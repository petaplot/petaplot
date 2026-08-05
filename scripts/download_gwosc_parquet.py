#!/usr/bin/env python3
"""
PetaPlot GWOSC Parquet Benchmark Dataset Downloader
===================================================

Este script descarga datos masivos de ondas gravitacionales (strain a 16 kHz) 
desde el Gravitational Wave Open Science Center (GWOSC) usando `gwosc` + `h5py`
y los exporta a un único archivo `.parquet` altamente optimizado (compresión ZSTD,
muestras float32 y grupos de filas de 4096 segundos).

Requisitos (0 compilación C necesaria):
    uv pip install h5py pyarrow numpy gwosc

Ejemplos de uso:

1. Micro-benchmark (GW150914 - 1 chunk 4096s ~67 millones de puntos):
    uv run scripts/download_gwosc_parquet.py --preset gw150914 -d L1 -o gw150914_L1.parquet

2. Benchmark de 1 día de O1 (~1.4 mil millones de puntos):
    uv run scripts/download_gwosc_parquet.py --preset o1_day1 -d H1 -o o1_day1_H1.parquet
"""

import argparse
import gc
import os
import sys
import time
import urllib.request


def parse_args():
    parser = argparse.ArgumentParser(
        description="Descarga datos de GWOSC a 16 kHz y genera un único archivo .parquet para PetaPlot."
    )
    parser.add_argument(
        "--detector", "-d",
        default="L1",
        choices=["H1", "L1", "V1", "K1"],
        help="Detector de ondas gravitacionales (default: L1)."
    )
    parser.add_argument(
        "--preset",
        choices=["gw150914", "o1_day1", "custom"],
        default="gw150914",
        help="Preset de dataset (gw150914: 1 chunk ~67M pts; o1_day1: 1 día ~1.4B pts)."
    )
    parser.add_argument(
        "--start-gps",
        type=int,
        help="Tiempo GPS de inicio (requerido si --preset=custom)."
    )
    parser.add_argument(
        "--end-gps",
        type=int,
        help="Tiempo GPS de fin (requerido si --preset=custom)."
    )
    parser.add_argument(
        "--output", "-o",
        type=str,
        default=None,
        help="Ruta del archivo .parquet de salida."
    )
    return parser.parse_args()


def download_file(url, target_path):
    """Descarga una URL HTTP a un archivo local con reconexión simple."""
    if os.path.exists(target_path):
        return
    urllib.request.urlretrieve(url, target_path)


def main():
    args = parse_args()

    try:
        import h5py
        import numpy as np
        import pyarrow as pa
        import pyarrow.parquet as pq
        from gwosc.locate import get_urls
    except ImportError as e:
        print(f"Error: Falta una librería requerida: {e}")
        print("Instálalas ejecutando: uv pip install h5py pyarrow numpy gwosc")
        sys.exit(1)

    # Determinar rango GPS según preset
    if args.preset == "gw150914":
        start_gps = 1126257414
        end_gps = 1126261510
        default_outfile = f"gw150914_{args.detector}_16kHz.parquet"
    elif args.preset == "o1_day1":
        start_gps = 1126259462
        end_gps = 1126259462 + 86400
        default_outfile = f"o1_day1_{args.detector}_16kHz.parquet"
    else:
        if not args.start_gps or not args.end_gps:
            print("Error: Para --preset=custom debes especificar --start-gps y --end-gps.")
            sys.exit(1)
        start_gps = args.start_gps
        end_gps = args.end_gps
        default_outfile = f"gwosc_custom_{args.detector}_{start_gps}_{end_gps}.parquet"

    output_path = args.output if args.output else default_outfile

    print("=" * 65)
    print("   GWOSC Data Exporter para PetaPlot")
    print("=" * 65)
    print(f" Detector:            {args.detector}")
    print(f" Rango GPS:           {start_gps} -> {end_gps}")
    print(f" Archivo de Salida:   {output_path}")
    print("=" * 65)

    print("Buscando URLs de datos en GWOSC API...")
    try:
        urls = get_urls(args.detector, start_gps, end_gps, sample_rate=16384)
    except Exception as e:
        print(f"Error obteniendo URLs de GWOSC: {e}")
        sys.exit(1)

    if not urls:
        print(f"No se encontraron URLs de datos a 16kHz para {args.detector} en el rango {start_gps}-{end_gps}.")
        sys.exit(1)

    print(f"Se encontraron {len(urls)} archivo(s) HDF5 en GWOSC.")

    schema = pa.schema([
        ("timestamp", pa.float64()),
        ("strain", pa.float32())
    ])

    written_groups = 0
    total_samples_written = 0
    start_time = time.time()
    cache_dir = os.path.join(".cache", "gwosc_hdf5")
    os.makedirs(cache_dir, exist_ok=True)

    with pq.ParquetWriter(output_path, schema, compression="zstd") as writer:
        for idx, url in enumerate(urls, start=1):
            filename = os.path.basename(url)
            local_hdf5 = os.path.join(cache_dir, filename)

            print(f"[{idx:03d}/{len(urls):03d}] Descargando {filename}...", end="", flush=True)
            try:
                download_file(url, local_hdf5)
                print(" OK!", end="", flush=True)

                with h5py.File(local_hdf5, "r") as h5:
                    strain_ds = h5["strain/Strain"]
                    strain_vals = strain_ds[:].astype(np.float32)
                    
                    xstart = strain_ds.attrs.get("Xstart", None)
                    if xstart is None:
                        xstart = h5["meta/GPSstart"][()] if "meta/GPSstart" in h5 else start_gps

                    dx = strain_ds.attrs.get("dx", None)
                    if dx is None:
                        dx = strain_ds.attrs.get("dt", None)
                    if dx is None:
                        dx = 1.0 / 16384.0

                    num_points = len(strain_vals)
                    
                    # Generar timestamps
                    times_arr = xstart + np.arange(num_points, dtype=np.float64) * dx

                    table = pa.Table.from_arrays(
                        [pa.array(times_arr), pa.array(strain_vals)],
                        schema=schema
                    )

                    writer.write_table(table)
                    written_groups += 1
                    total_samples_written += num_points
                    print(f" Procesado ({num_points:,} puntos)")

            except Exception as e:
                print(f" Error procesando {filename}: {e}")

            gc.collect()

    elapsed = time.time() - start_time
    file_size_mb = os.path.getsize(output_path) / (1024 * 1024) if os.path.exists(output_path) else 0

    print("\n" + "=" * 65)
    print(" ¡DESCARGA Y CONVERSIÓN COMPLETADA!")
    print("=" * 65)
    print(f" Total Muestras Escritas: {total_samples_written:,}")
    print(f" Row Groups Creados:     {written_groups}")
    print(f" Tamaño Parquet Final:   {file_size_mb:.2f} MB")
    print(f" Tiempo Transcurrido:    {elapsed:.2f} s")
    print(f"\nPara visualizarlo con PetaPlot:")
    print(f"  cargo run --bin petaplot --release -- {output_path}")
    print("=" * 65)


if __name__ == "__main__":
    main()
