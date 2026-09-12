use crate::{cli::dotenv, models::SecretInput};
use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use serde::de::{self, DeserializeSeed, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::{collections::BTreeMap, fmt, fs, io::Read, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum SecretFormat {
  Dotenv,
  Json,
  Yaml,
}

impl SecretFormat {
  pub fn for_input(
    path: &Path,
    explicit: Option<Self>,
  ) -> Result<Self> {
    if let Some(format) = explicit {
      return Ok(format);
    }
    if path == Path::new("-") {
      bail!("--format is required when reading secrets from stdin");
    }
    Ok(Self::from_path(path))
  }

  pub fn for_output(
    path: Option<&Path>,
    explicit: Option<Self>,
  ) -> Self {
    explicit.unwrap_or_else(|| path.map_or(Self::Dotenv, Self::from_path))
  }

  fn from_path(path: &Path) -> Self {
    match path
      .extension()
      .and_then(|extension| extension.to_str())
      .map(str::to_ascii_lowercase)
      .as_deref()
    {
      Some("json") => Self::Json,
      Some("yaml" | "yml") => Self::Yaml,
      _ => Self::Dotenv,
    }
  }
}

pub fn read(
  path: &Path,
  explicit: Option<SecretFormat>,
) -> Result<Vec<SecretInput>> {
  let format = SecretFormat::for_input(path, explicit)?;
  let text = if path == Path::new("-") {
    let mut text = String::new();
    std::io::stdin()
      .read_to_string(&mut text)
      .context("failed to read secrets from stdin")?;
    text
  } else {
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?
  };
  parse(&text, format)
}

pub fn parse(
  text: &str,
  format: SecretFormat,
) -> Result<Vec<SecretInput>> {
  if text.trim().is_empty() {
    bail!("the input contains no secret entries");
  }
  let entries = match format {
    SecretFormat::Dotenv => dotenv::parse(text)?,
    SecretFormat::Json => {
      let mut deserializer = serde_json::Deserializer::from_str(text);
      let values = StrictSecretMap::deserialize(&mut deserializer).context("invalid JSON input")?;
      deserializer.end().context("invalid JSON input")?;
      values.into_entries()
    }
    SecretFormat::Yaml => {
      let options = serde_saphyr::options! {
        duplicate_keys: serde_saphyr::DuplicateKeyPolicy::Error,
        merge_keys: serde_saphyr::MergeKeyPolicy::Error,
        no_schema: true,
        with_snippet: false,
      };
      serde_saphyr::from_str_with_options::<StrictSecretMap>(text, options)
        .context("invalid YAML input")?
        .into_entries()
    }
  };
  if entries.is_empty() {
    bail!("the input contains no secret entries");
  }
  Ok(entries)
}

pub fn render(
  entries: &[SecretInput],
  format: SecretFormat,
) -> Result<String> {
  let sorted = entries
    .iter()
    .map(|entry| (entry.key.as_str(), entry.value.as_str()))
    .collect::<BTreeMap<_, _>>();
  match format {
    SecretFormat::Dotenv => {
      let entries = sorted
        .into_iter()
        .map(|(key, value)| SecretInput {
          key: key.into(),
          value: value.into(),
        })
        .collect::<Vec<_>>();
      Ok(dotenv::render(&entries))
    }
    SecretFormat::Json => Ok(format!("{}\n", serde_json::to_string_pretty(&sorted)?)),
    SecretFormat::Yaml => Ok(serde_saphyr::to_string(&sorted)?),
  }
}

struct StrictSecretMap(BTreeMap<String, String>);

impl StrictSecretMap {
  fn into_entries(self) -> Vec<SecretInput> {
    self
      .0
      .into_iter()
      .map(|(key, value)| SecretInput { key, value })
      .collect()
  }
}

impl<'de> Deserialize<'de> for StrictSecretMap {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_map(SecretMapVisitor)
  }
}

struct SecretMapVisitor;

impl<'de> Visitor<'de> for SecretMapVisitor {
  type Value = StrictSecretMap;

  fn expecting(
    &self,
    formatter: &mut fmt::Formatter,
  ) -> fmt::Result {
    formatter.write_str("a flat mapping of secret names to string values")
  }

  fn visit_map<A>(
    self,
    mut map: A,
  ) -> std::result::Result<Self::Value, A::Error>
  where
    A: MapAccess<'de>,
  {
    let mut entries = BTreeMap::new();
    while let Some(key) = map.next_key_seed(SecretKeySeed)? {
      if key.is_empty() {
        return Err(de::Error::custom("secret keys may not be empty"));
      }
      if entries.contains_key(&key) {
        return Err(de::Error::custom(format!("duplicate secret key {key:?}")));
      }
      let value = map.next_value_seed(SecretValueSeed { key: &key })?;
      entries.insert(key, value);
    }
    Ok(StrictSecretMap(entries))
  }
}

struct SecretKeySeed;

impl<'de> DeserializeSeed<'de> for SecretKeySeed {
  type Value = String;

  fn deserialize<D>(
    self,
    deserializer: D,
  ) -> std::result::Result<Self::Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_any(StringOnlyVisitor {
      label: "secret key",
    })
  }
}

struct SecretValueSeed<'a> {
  key: &'a str,
}

impl<'de> DeserializeSeed<'de> for SecretValueSeed<'_> {
  type Value = String;

  fn deserialize<D>(
    self,
    deserializer: D,
  ) -> std::result::Result<Self::Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_any(StringOnlyVisitor { label: self.key })
  }
}

struct StringOnlyVisitor<'a> {
  label: &'a str,
}

impl<'de> Visitor<'de> for StringOnlyVisitor<'_> {
  type Value = String;

  fn expecting(
    &self,
    formatter: &mut fmt::Formatter,
  ) -> fmt::Result {
    write!(formatter, "a string for {:?}", self.label)
  }

  fn visit_str<E>(
    self,
    value: &str,
  ) -> std::result::Result<Self::Value, E> {
    Ok(value.into())
  }

  fn visit_string<E>(
    self,
    value: String,
  ) -> std::result::Result<Self::Value, E> {
    Ok(value)
  }

  fn visit_bool<E>(
    self,
    _value: bool,
  ) -> std::result::Result<Self::Value, E>
  where
    E: de::Error,
  {
    Err(self.wrong_type("boolean"))
  }

  fn visit_i64<E>(
    self,
    _value: i64,
  ) -> std::result::Result<Self::Value, E>
  where
    E: de::Error,
  {
    Err(self.wrong_type("number"))
  }

  fn visit_u64<E>(
    self,
    _value: u64,
  ) -> std::result::Result<Self::Value, E>
  where
    E: de::Error,
  {
    Err(self.wrong_type("number"))
  }

  fn visit_f64<E>(
    self,
    _value: f64,
  ) -> std::result::Result<Self::Value, E>
  where
    E: de::Error,
  {
    Err(self.wrong_type("number"))
  }

  fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
  where
    E: de::Error,
  {
    Err(self.wrong_type("null"))
  }

  fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
  where
    E: de::Error,
  {
    Err(self.wrong_type("null"))
  }

  fn visit_seq<A>(
    self,
    _sequence: A,
  ) -> std::result::Result<Self::Value, A::Error>
  where
    A: de::SeqAccess<'de>,
  {
    Err(self.wrong_type("array"))
  }

  fn visit_map<A>(
    self,
    _map: A,
  ) -> std::result::Result<Self::Value, A::Error>
  where
    A: MapAccess<'de>,
  {
    Err(self.wrong_type("object"))
  }
}

impl StringOnlyVisitor<'_> {
  fn wrong_type<E: de::Error>(
    &self,
    kind: &str,
  ) -> E {
    de::Error::custom(format!("{:?} must be a string, found {kind}", self.label))
  }
}
