use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Episode {
  pub id: i64,
  pub created_at: i64,
  pub summary_text: String,
  pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone)]
pub struct MemoryMatch {
  pub episode: Episode,
  pub similarity: f32,
}

#[derive(Debug, Error)]
pub enum MemoryStoreError {
  #[error("SQLite error: {0}")]
  Sqlite(#[from] rusqlite::Error),

  #[error("system clock error: {0}")]
  Clock(#[from] std::time::SystemTimeError),

  #[error("invalid embedding blob length: {0} bytes")]
  InvalidEmbeddingLength(usize),

  #[error("embedding dimensions do not match")]
  DimensionMismatch,

  #[error("embedding must not be empty")]
  EmptyEmbedding,
}

pub struct MemoryStore {
  connection: Mutex<Connection>,
}

impl MemoryStore {
  pub fn open(path: impl AsRef<Path>) -> Result<Self, MemoryStoreError> {
    let connection = Connection::open(path)?;
    Self::from_connection(connection)
  }

  pub fn in_memory() -> Result<Self, MemoryStoreError> {
    let connection = Connection::open_in_memory()?;
    Self::from_connection(connection)
  }

  fn from_connection(connection: Connection) -> Result<Self, MemoryStoreError> {
    connection.execute_batch(
      "\
      CREATE TABLE IF NOT EXISTS episodes (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        created_at INTEGER NOT NULL,
        summary_text TEXT NOT NULL,
        embedding BLOB
      );
      ",
    )?;

    Ok(Self {
      connection: Mutex::new(connection),
    })
  }

  fn encode_embedding(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);

    for value in embedding {
      bytes.extend_from_slice(&value.to_le_bytes());
    }

    bytes
  }

  fn decode_embedding(bytes: &[u8]) -> Result<Vec<f32>, MemoryStoreError> {
    if !bytes.len().is_multiple_of(4) {
      return Err(MemoryStoreError::InvalidEmbeddingLength(bytes.len()));
    }

    Ok(
      bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect(),
    )
  }

  fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.len() != right.len() || left.is_empty() {
      return None;
    }

    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;

    for (&left_value, &right_value) in left.iter().zip(right.iter()) {
      dot += left_value * right_value;
      left_norm += left_value * left_value;
      right_norm += right_value * right_value;
    }

    if left_norm == 0.0 || right_norm == 0.0 {
      return None;
    }

    Some(dot / (left_norm.sqrt() * right_norm.sqrt()))
  }

  pub fn store_episode(
    &self,
    summary_text: &str,
    embedding: Option<&[f32]>,
  ) -> Result<i64, MemoryStoreError> {
    let created_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let embedding_blob = embedding.map(Self::encode_embedding);

    let connection = self
      .connection
      .lock()
      .expect("memory store mutex should not be poisoned");

    connection.execute(
      "\
      INSERT INTO episodes (created_at, summary_text, embedding)
      VALUES (?1, ?2, ?3)
      ",
      params![created_at, summary_text, embedding_blob],
    )?;

    Ok(connection.last_insert_rowid())
  }

  pub fn search_by_keyword(&self, keyword: &str) -> Result<Vec<Episode>, MemoryStoreError> {
    let connection = self
      .connection
      .lock()
      .expect("memory store mutex should not be poisoned");

    let pattern = format!("%{keyword}%");

    let mut statement = connection.prepare(
      "\
      SELECT id, created_at, summary_text, embedding
      FROM episodes
      WHERE summary_text LIKE ?1
      ORDER BY created_at DESC
      ",
    )?;

    let rows = statement.query_map(params![pattern], |row| {
      let embedding = row
        .get::<_, Option<Vec<u8>>>(3)?
        .map(|bytes| Self::decode_embedding(&bytes));

      Ok((
        row.get::<_, i64>(0)?,
        row.get::<_, i64>(1)?,
        row.get::<_, String>(2)?,
        embedding,
      ))
    })?;

    let mut episodes = Vec::new();

    for row in rows {
      let (id, created_at, summary_text, embedding) = row?;

      let embedding = match embedding {
        Some(Ok(embedding)) => Some(embedding),
        Some(Err(error)) => return Err(error),
        None => None,
      };

      episodes.push(Episode {
        id,
        created_at,
        summary_text,
        embedding,
      });
    }

    Ok(episodes)
  }
}
