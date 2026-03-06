mod vault;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use uuid::Uuid;

use vault::VaultSession;

#[derive(Parser)]
#[command(name = "pn", about = "PortableNote CLI")]
struct Cli {
    /// Path to the vault directory. If omitted, searches upward from cwd.
    #[arg(long, global = true)]
    vault: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new empty vault.
    Init {
        /// Directory to create the vault in (defaults to cwd).
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Add a new block to the vault.
    Add {
        /// Block name.
        name: String,
        /// Block content (optional, reads from stdin if omitted).
        #[arg(short, long, default_value = "")]
        content: String,
    },

    /// List all blocks in the vault.
    List,

    /// Rename a block.
    Rename {
        /// Block UUID.
        id: Uuid,
        /// New name.
        name: String,
    },

    /// Update a block's content.
    Edit {
        /// Block UUID.
        id: Uuid,
        /// New content.
        content: String,
    },

    /// Delete a block.
    Delete {
        /// Block UUID.
        id: Uuid,
        /// Force deletion even with incoming edges.
        #[arg(long)]
        cascade: bool,
    },

    /// Add a reference edge between two blocks.
    Link {
        /// Source block UUID.
        source: Uuid,
        /// Target block UUID.
        target: Uuid,
    },

    /// Remove a reference edge.
    Unlink {
        /// Edge UUID.
        id: Uuid,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Init { path } => {
            VaultSession::init(&path)?;
            println!("initialized vault at {}", path.display());
        }
        command => {
            let vault_path = VaultSession::resolve_vault_path(cli.vault.as_deref())?;
            let mut session = VaultSession::open(&vault_path)?;

            match command {
                Command::Add { name, content } => {
                    session.add_block(&name, &content)?;
                }
                Command::List => {
                    session.list_blocks();
                }
                Command::Rename { id, name } => {
                    session.rename_block(id, &name)?;
                }
                Command::Edit { id, content } => {
                    session.mutate_content(id, &content)?;
                }
                Command::Delete { id, cascade } => {
                    session.delete_block(id, cascade)?;
                }
                Command::Link { source, target } => {
                    session.add_edge(source, target)?;
                }
                Command::Unlink { id } => {
                    session.remove_edge(id)?;
                }
                Command::Init { .. } => unreachable!(),
            }
        }
    }

    Ok(())
}
