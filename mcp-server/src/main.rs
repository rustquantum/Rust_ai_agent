use chrono::{DateTime, Utc};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{ErrorData as McpError, *},
    schemars, serde_json, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::borrow::Cow;
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
// Modèle de données correspondant à la table SQLite
#[derive(Debug, FromRow, Clone, Deserialize, Serialize, schemars::JsonSchema)]
struct Client {
    #[schemars(description = "The data client to get from database")]
    id: i64, // SQLite utilise des entiers 64 bits signés
    nom: String,
    entreprise: Option<String>,
    email: String,
    telephone: Option<String>,
    date: DateTime<Utc>,
}

// Structure pour la mise à jour sélective d'un client
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct UpdateClient {
    #[schemars(description = "Data client to update database")]
    nom: String,
    entreprise: Option<String>,
    email: String,
    telephone: Option<String>,
}

///structure pour la creation
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateClientInput {
    #[schemars(description = "Data client to post in database")]
    nom: String,
    email: String,
    entreprise: Option<String>,
    telephone: Option<String>,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct Ident {
    id: i64,
}
#[derive(Clone)]
struct ClientRepository {
    pool: Arc<SqlitePool>,
}
#[tool_router]
impl ClientRepository {
    fn new(pool: SqlitePool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    // --- CREATE ---
    #[tool(description = "Perform or post or create resource of data client in database")]
    async fn create(
        &self,
        Parameters(input): Parameters<CreateClientInput>,
    ) -> Result<CallToolResult, McpError> {
        let client = sqlx::query_as::<_, Client>(
            r#"
            INSERT INTO clients (nom, entreprise, email, telephone)
            VALUES (?, ?, ?, ?) RETURNING id,nom,email,entreprise,telephone,date
            "#,
        )
        .bind(input.nom)
        .bind(input.email)
        .bind(input.entreprise)
        .bind(input.telephone)
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Fail to post data:{}", e)),
            ..Default::default()
        })?;
        let ouput = serde_json::to_string(&client).unwrap();
        Ok(CallToolResult::success(vec![ContentBlock::Text(
            TextContent::new(ouput),
        )]))
    }

    // --- READ (Tout lister) ---
    #[tool(description = "get all clients from database")]
    async fn find_all(&self) -> Result<CallToolResult, McpError> {
        let clients = sqlx::query_as::<_, Client>(
            "SELECT id, nom, entreprise, email, telephone, data FROM clients ORDER BY data DESC",
        )
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Fail to post data:{}", e)),
            ..Default::default()
        })?;
        let ouput = serde_json::to_string(&clients).unwrap();
        Ok(CallToolResult::success(vec![ContentBlock::Text(
            TextContent::new(ouput),
        )]))
    }

    // --- READ (Trouver par ID) ---
    #[tool(description = "get or data'client who are identify by x from database")]
    async fn find_by_id(
        &self,
        Parameters(ident): Parameters<Ident>,
    ) -> Result<CallToolResult, McpError> {
        let client = sqlx::query_as::<_, Client>(
            "SELECT id, nom, entreprise, email, telephone, date FROM clients WHERE id = ?",
        )
        .bind(ident.id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Fail to post data:{}", e)),
            ..Default::default()
        })?;
        let ouput = serde_json::to_string(&client).unwrap();
        Ok(CallToolResult::success(vec![ContentBlock::Text(
            TextContent::new(ouput),
        )]))
    }

    // --- UPDATE ---
    #[tool(description = "update data'client who is identify by x number in database")]
    async fn update(
        &self,
        Parameters(ident): Parameters<Ident>,
        Parameters(input): Parameters<UpdateClient>,
    ) -> Result<CallToolResult, McpError> {
        let client = sqlx::query_as::<_, Client>(
            r#"
            UPDATE clients 
            SET nom = ?, entreprise = ?, email = ?, telephone = ?
            WHERE id = ?
            "#,
        )
        .bind(ident.id)
        .bind(input.nom)
        .bind(input.email)
        .bind(input.entreprise)
        .bind(input.telephone)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Fail to post data:{}", e)),
            ..Default::default()
        })?;
        let ouput = serde_json::to_string(&client).unwrap();
        Ok(CallToolResult::success(vec![ContentBlock::Text(
            TextContent::new(ouput),
        )]))
    }

    // --- DELETE ---
    #[tool(description = "delete data'client who is idetify by x number on database")]
    async fn delete(
        &self,
        Parameters(ident): Parameters<Ident>,
    ) -> Result<CallToolResult, McpError> {
        let result = sqlx::query(r#"DELETE FROM clients WHERE id = ?"#)
            .bind(ident.id)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| McpError {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Fail to post data:{}", e)),
                ..Default::default()
            })?
            .rows_affected();
        let ouput = serde_json::to_string(&result).unwrap();
        Ok(CallToolResult::success(vec![ContentBlock::Text(
            TextContent::new(ouput),
        )]))
    }
}
#[tool_handler]
impl ServerHandler for ClientRepository {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //let database_url = "sqlite:clients.db";
    let connection_options = SqliteConnectOptions::from_str("sqlite::memory:")? // Ou "sqlite://database.db"
        .create_if_missing(true);
    // 1. Initialisation de la connexion SQLite (Crée le fichier s'il n'existe pas)
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await
        .expect("Fail to connect to sqlite database");
    // 2. Initialisation de la table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS clients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nom TEXT NOT NULL,
            entreprise TEXT,
            email TEXT NOT NULL UNIQUE,
            telephone TEXT,
            cree_le TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await?;

    println!("--- Base de données initialisée avec succès ---\n");

    let service = ClientRepository::new(pool).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
