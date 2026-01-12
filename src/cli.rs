use clap::Parser;

/// A CLI tool to read/modify redb database files
#[derive(Parser, Debug, Clone)]
#[command(version)]
pub struct CliArgs {
    /// Database file
    #[arg(required = true)]
    pub file: String,

    /// Table name
    pub table: Option<String>,

    /// Key (raw string or JSON value)
    pub key: Option<String>,

    /// Value (raw string or JSON value)
    pub value: Option<String>,

    /// List tables and types
    #[arg(short, long, conflicts_with = "remove")]
    pub list: bool,

    /// Create database file and table
    #[arg(short, long)]
    pub create: bool,

    /// Remove key
    #[arg(short, long, requires = "key")]
    pub remove: bool,

    /// Delete table
    #[arg(short, long, requires = "table", conflicts_with = "key")]
    pub delete: bool,

    /// Open as multimap
    #[arg(short, long)]
    pub multimap: bool,

    /// Output JSON
    #[arg(short, long)]
    pub json: bool,

    /// Table schema, e.g. String -> String
    #[arg(long)]
    pub schema: Option<String>,

    /// Open database read-only
    #[arg(long, alias = "read-only")]
    pub ro: bool,

    /// Show table stats
    #[arg(long, conflicts_with = "list")]
    pub stats: bool,

    /// Check integrity
    #[arg(long)]
    pub check: bool,

    /// Compact database
    #[arg(long)]
    pub compact: bool,
}
