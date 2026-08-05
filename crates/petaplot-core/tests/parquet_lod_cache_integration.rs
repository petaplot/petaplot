use arrow_array::{Float32Array, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::arrow_writer::ArrowWriter;
use petaplot_core::lod::builder::LodBuilder;
use petaplot_core::lod::cache::LodCache;
use petaplot_core::storage::parquet_reader::ParquetReader;
use std::fs::File;
use std::sync::Arc;

#[test]
fn test_parquet_to_lod_cache_end_to_end() -> petaplot_core::error::Result<()> {
    let temp_dir = std::env::temp_dir();
    let parquet_file = temp_dir.join("test_e2e_signal.parquet");
    let cache_dir = temp_dir.join("test_petaplot_cache");

    // 1. Generar archivo Parquet de prueba con 200,000 puntos
    let num_samples = 200_000;
    let schema = Arc::new(Schema::new(vec![Field::new("strain", DataType::Float32, false)]));
    let values: Vec<f32> = (0..num_samples).map(|i| (i as f32 * 0.01).sin()).collect();
    let array = Float32Array::from(values);
    let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(array)]).unwrap();

    {
        let file = File::create(&parquet_file)?;
        let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
    }

    // 2. Probar Cache Miss: Leer Parquet y generar pirámide LOD
    let reader = ParquetReader::open(&parquet_file)?;
    assert_eq!(reader.num_rows(), num_samples);

    let builder = LodBuilder::new().with_factor(10);
    let pyramid = builder.build_from_parquet(&reader, Some("strain"))?;
    assert!(pyramid.num_levels() >= 3);
    assert_eq!(pyramid.total_samples, num_samples);

    // 3. Guardar en Caché LOD independiente
    let cache_key = LodCache::compute_cache_key(&parquet_file);
    let cache_path = cache_dir.join(cache_key);
    LodCache::save_to_cache(&pyramid, &cache_path)?;
    assert!(cache_path.exists());

    // 4. Probar Cache Hit: Cargar Pirámide LOD instantáneamente desde el archivo `.lod`
    let loaded_pyramid = LodCache::load_from_cache(&cache_path)?;
    assert_eq!(loaded_pyramid.total_samples, num_samples);
    assert_eq!(loaded_pyramid.num_levels(), pyramid.num_levels());
    assert_eq!(loaded_pyramid.levels[0].pairs.len(), pyramid.levels[0].pairs.len());

    // Limpieza
    let _ = std::fs::remove_file(parquet_file);
    let _ = std::fs::remove_file(cache_path);
    let _ = std::fs::remove_dir_all(cache_dir);

    println!("¡Prueba de integración de Parquet + Caché LOD completada con éxito!");
    Ok(())
}
