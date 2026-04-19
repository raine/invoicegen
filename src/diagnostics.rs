use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config file {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid config file {path}: {message}")]
    Parse { path: PathBuf, message: String },
}

#[derive(Debug, Error)]
pub enum InvoiceInputError {
    #[error("could not read invoice from stdin")]
    ReadStdin {
        #[source]
        source: std::io::Error,
    },
    #[error("could not read invoice file {path}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid invoice {path}: {message}")]
    Parse { path: String, message: String },
}

#[derive(Debug, Error)]
pub enum MergeError {
    #[error("unknown client '{client}' in invoice input")]
    UnknownClient {
        client: String,
        available: Vec<String>,
    },
    #[error("missing {field}")]
    MissingField {
        field: &'static str,
        help: Option<String>,
    },
    #[error("invoice must include at least one item")]
    NoItems,
    #[error("invalid tax_rate: must be zero or greater")]
    NegativeTaxRate,
    #[error("item #{index} is missing {field}")]
    MissingItemField {
        index: usize,
        field: &'static str,
        help: Option<String>,
    },
    #[error("item #{index} has a negative {field}")]
    NegativeItemValue { index: usize, field: &'static str },
    #[error("item #{index} description is empty")]
    EmptyItemDescription { index: usize },
}

impl MergeError {
    pub fn details(&self) -> Option<String> {
        match self {
            Self::UnknownClient { available, .. } if !available.is_empty() => {
                Some(format!("available clients: {}", available.join(", ")))
            }
            _ => None,
        }
    }

    pub fn help(&self) -> Option<&str> {
        match self {
            Self::UnknownClient { available, .. } => {
                if available.is_empty() {
                    Some(
                        "Define the client inline in the invoice or add it under clients in your config file.",
                    )
                } else {
                    Some(
                        "Use one of the configured client keys shown above, or define client fields inline in the invoice.",
                    )
                }
            }
            Self::MissingField { help, .. } => help.as_deref(),
            Self::NoItems => Some("Add at least one entry under items."),
            Self::NegativeTaxRate => Some("Set tax_rate to 0 or a positive number."),
            Self::MissingItemField { help, .. } => help.as_deref(),
            Self::NegativeItemValue { field, .. } => match *field {
                "quantity" => Some("Set the quantity to 0 or a positive number."),
                "rate" => Some("Set the rate to 0 or a positive number."),
                _ => None,
            },
            Self::EmptyItemDescription { .. } => Some("Give the item a non-empty description."),
        }
    }
}

#[derive(Debug, Error)]
pub enum LineItemInputError {
    #[error("item must be 'DESCRIPTION: QUANTITY [@ RATE]'")]
    MissingSeparator,
    #[error("item description is empty")]
    EmptyDescription,
    #[error("invalid item quantity '{value}'")]
    InvalidQuantity {
        value: String,
        #[source]
        source: rust_decimal::Error,
    },
    #[error("invalid item rate '{value}'")]
    InvalidRate {
        value: String,
        #[source]
        source: rust_decimal::Error,
    },
}

impl LineItemInputError {
    pub fn help(&self) -> &'static str {
        "Use the format 'DESCRIPTION: QUANTITY [@ RATE]', for example 'Consulting: 10 @ 150'."
    }
}

#[derive(Debug, Error)]
pub enum PathsError {
    #[error("invoice path {path} has no parent directory")]
    MissingParent { path: PathBuf },
}

#[derive(Debug, Error)]
pub enum GenerateError {
    #[error("could not determine the base directory for invoice input {path}")]
    InvoiceBaseDir {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },
    #[error("could not determine the current directory for stdin invoice input")]
    StdinBaseDir {
        #[source]
        source: anyhow::Error,
    },
    #[error("could not read logo file {path}")]
    ReadLogo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not create output directory {path}")]
    CreateOutputDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write output PDF to {path}")]
    WriteOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum PresentError {
    #[error("invalid date_format '{format}'")]
    InvalidDateFormat {
        format: String,
        #[source]
        source: jiff::Error,
    },
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("could not prepare invoice data for rendering")]
    SerializeContext {
        #[source]
        source: serde_json::Error,
    },
    #[error("Typst failed to compile the invoice template")]
    Compile { details: String },
    #[error("Typst failed to export the invoice PDF")]
    Pdf { details: String },
}

impl RenderError {
    pub fn details(&self) -> Option<&str> {
        match self {
            Self::Compile { details } | Self::Pdf { details } => Some(details),
            Self::SerializeContext { .. } => None,
        }
    }

    pub fn help(&self) -> Option<&str> {
        match self {
            Self::SerializeContext { .. } => None,
            Self::Compile { .. } | Self::Pdf { .. } => Some(
                "Check the invoice data and template inputs above. If you use a logo, confirm the file exists and Typst can read its format.",
            ),
        }
    }
}

pub fn format_yaml_path_error(error: serde_path_to_error::Error<serde_yml::Error>) -> String {
    let field_path = error.path().to_string();
    let inner = error.into_inner();
    let detail = inner.to_string();
    if field_path.is_empty() {
        detail
    } else {
        format!("at {field_path}: {detail}")
    }
}

pub fn display_input_path(path: &Path) -> String {
    if path == Path::new("-") {
        "stdin".to_string()
    } else {
        path.display().to_string()
    }
}
