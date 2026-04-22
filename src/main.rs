use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;

// ==================== CONFIGURACIÓN ====================

lazy_static::lazy_static! {
    static ref GITHUB_TOKEN: Option<String> = env::var("GITHUB_TOKEN").ok();
}

// ==================== STRUCTS DE RESPUESTA ====================

#[derive(Serialize)]
struct ApiResponse<T> {
    status: String,
    data: T,
    user_agent: String,
    fecha_servicio: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_configurado: Option<bool>,
}

#[derive(Deserialize)]
struct QueryParams {
    nombre: String,
}

// ==================== STRUCTS GITHUB GRAPHQL ====================

#[derive(Serialize)]
struct GraphQLRequest {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize, Debug)]
struct GraphQLError {
    message: String,
}

// Datos del usuario desde GraphQL
#[derive(Serialize, Deserialize, Debug)]
struct GitHubUserGraphQL {
    login: String,
    id: String,
    avatarUrl: String,
    url: String,
    name: Option<String>,
    company: Option<String>,
    location: Option<String>,
    email: Option<String>,
    bio: Option<String>,
    repositories: Option<GitHubRepoCount>,
    followers: Option<GitHubFollowerCount>,
    following: Option<GitHubFollowingCount>,
    createdAt: String,
    updatedAt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    starredRepositories: Option<GitHubRepoCount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contributionsCollection: Option<ContributionsCollection>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GitHubRepoCount {
    totalCount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GitHubFollowerCount {
    totalCount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct GitHubFollowingCount {
    totalCount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ContributionsCollection {
    totalCommitContributions: u64,
    totalPullRequestContributions: u64,
    totalIssueContributions: u64,
    restrictedContributionsCount: u64,
}

// Usuario simplificado para la respuesta
#[derive(Serialize, Debug)]
struct GitHubUser {
    login: String,
    id: String,
    avatar_url: String,
    html_url: String,
    name: Option<String>,
    company: Option<String>,
    location: Option<String>,
    email: Option<String>,
    bio: Option<String>,
    public_repos: u64,
    followers: u64,
    following: u64,
    created_at: String,
    updated_at: String,
    // Campos extendidos (solo con token)
    #[serde(skip_serializing_if = "Option::is_none")]
    private_repos: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    starred_repos: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_commits: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_prs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_issues: Option<u64>,
}

impl From<GitHubUserGraphQL> for GitHubUser {
    fn from(user: GitHubUserGraphQL) -> Self {
        GitHubUser {
            login: user.login,
            id: user.id,
            avatar_url: user.avatarUrl,
            html_url: user.url,
            name: user.name,
            company: user.company,
            location: user.location,
            email: user.email,
            bio: user.bio,
            public_repos: user.repositories.map(|r| r.totalCount).unwrap_or(0),
            followers: user.followers.map(|f| f.totalCount).unwrap_or(0),
            following: user.following.map(|f| f.totalCount).unwrap_or(0),
            created_at: user.createdAt,
            updated_at: user.updatedAt,
            private_repos: None,
            starred_repos: user.starredRepositories.map(|r| r.totalCount),
            total_commits: user.contributionsCollection.as_ref().map(|c| c.totalCommitContributions),
            total_prs: user.contributionsCollection.as_ref().map(|c| c.totalPullRequestContributions),
            total_issues: user.contributionsCollection.as_ref().map(|c| c.totalIssueContributions),
        }
    }
}

// Repositorio desde GraphQL
#[derive(Serialize, Deserialize, Debug)]
struct GitHubRepoGraphQL {
    name: String,
    description: Option<String>,
    url: String,
    primaryLanguage: Option<Language>,
    stargazerCount: u64,
    forkCount: u64,
    watchers: Option<WatcherCount>,
    issues: Option<IssueCount>,
    createdAt: String,
    updatedAt: String,
    pushedAt: Option<String>,
    isPrivate: bool,
    isFork: bool,
    isArchived: bool,
    // Campos extendidos
    diskUsage: Option<u64>,
    homepageUrl: Option<String>,
    licenseInfo: Option<LicenseInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Language {
    name: String,
    color: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WatcherCount {
    totalCount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct IssueCount {
    totalCount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct LicenseInfo {
    name: String,
    spdxId: String,
}

// Repositorio simplificado
#[derive(Serialize, Debug)]
struct GitHubRepo {
    name: String,
    description: Option<String>,
    html_url: String,
    language: Option<String>,
    language_color: Option<String>,
    stargazers_count: u64,
    forks_count: u64,
    watchers_count: u64,
    open_issues_count: u64,
    created_at: String,
    updated_at: String,
    pushed_at: Option<String>,
    is_private: bool,
    is_fork: bool,
    is_archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk_usage: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
}

impl From<GitHubRepoGraphQL> for GitHubRepo {
    fn from(repo: GitHubRepoGraphQL) -> Self {
        GitHubRepo {
            name: repo.name,
            description: repo.description,
            html_url: repo.url,
            language: repo.primaryLanguage.as_ref().map(|l| l.name.clone()),
            language_color: repo.primaryLanguage.and_then(|l| l.color),
            stargazers_count: repo.stargazerCount,
            forks_count: repo.forkCount,
            watchers_count: repo.watchers.map(|w| w.totalCount).unwrap_or(0),
            open_issues_count: repo.issues.map(|i| i.totalCount).unwrap_or(0),
            created_at: repo.createdAt,
            updated_at: repo.updatedAt,
            pushed_at: repo.pushedAt,
            is_private: repo.isPrivate,
            is_fork: repo.isFork,
            is_archived: repo.isArchived,
            disk_usage: repo.diskUsage,
            homepage: repo.homepageUrl,
            license: repo.licenseInfo.map(|l| l.name),
        }
    }
}

// ==================== HELPERS ====================

fn get_user_agent(req: &HttpRequest) -> String {
    req.headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Desconocido")
        .to_string()
}

fn build_response<T: Serialize>(req: &HttpRequest, data: T) -> HttpResponse {
    let response = ApiResponse {
        status: "success".to_string(),
        data,
        user_agent: get_user_agent(req),
        fecha_servicio: Utc::now().to_rfc3339(),
        token_configurado: Some(GITHUB_TOKEN.is_some()),
    };
    HttpResponse::Ok().json(response)
}

fn build_error_response<T: Serialize>(req: &HttpRequest, status: actix_web::http::StatusCode, data: T) -> HttpResponse {
    let response = ApiResponse {
        status: "error".to_string(),
        data,
        user_agent: get_user_agent(req),
        fecha_servicio: Utc::now().to_rfc3339(),
        token_configurado: Some(GITHUB_TOKEN.is_some()),
    };
    HttpResponse::build(status).json(response)
}

fn get_github_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", "RustAPI-GraphQL/1.0".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    
    if let Some(ref token) = *GITHUB_TOKEN {
        headers.insert("Authorization", format!("Bearer {}", token).parse().unwrap());
    }
    
    headers
}

// ==================== CLIENTE GRAPHQL ====================

async fn graphql_query<T: for<'de> Deserialize<'de>>(
    query: &str,
    variables: Option<serde_json::Value>,
) -> Result<T, String> {
    let client = reqwest::Client::new();
    
    let request = GraphQLRequest {
        query: query.to_string(),
        variables,
    };

    let response = client
        .post("https://api.github.com/graphql")
        .headers(get_github_headers())
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Error de conexión: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Error HTTP {}: {}", status, body));
    }

    let graphql_response: GraphQLResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("Error parseando JSON: {}", e))?;

    if let Some(errors) = graphql_response.errors {
        return Err(errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join(", "));
    }

    graphql_response.data.ok_or_else(|| "No se recibieron datos".to_string())
}

// ==================== QUERIES GRAPHQL ====================

// Query básica sin token (información pública)
const QUERY_USER_BASIC: &str = r#"
query($username: String!) {
    user(login: $username) {
        login
        id
        avatarUrl
        url
        name
        company
        location
        email
        bio
        repositories { totalCount }
        followers { totalCount }
        following { totalCount }
        createdAt
        updatedAt
    }
}
"#;

// Query extendida con token (información completa)
const QUERY_USER_EXTENDED: &str = r#"
query($username: String!) {
    user(login: $username) {
        login
        id
        avatarUrl
        url
        name
        company
        location
        email
        bio
        repositories { totalCount }
        followers { totalCount }
        following { totalCount }
        starredRepositories { totalCount }
        createdAt
        updatedAt
        contributionsCollection {
            totalCommitContributions
            totalPullRequestContributions
            totalIssueContributions
            restrictedContributionsCount
        }
    }
}
"#;

// Query para repositorios
const QUERY_USER_REPOS: &str = r#"
query($username: String!, $first: Int!) {
    user(login: $username) {
        repositories(first: $first, orderBy: {field: UPDATED_AT, direction: DESC}) {
            nodes {
                name
                description
                url
                primaryLanguage {
                    name
                    color
                }
                stargazerCount
                forkCount
                watchers { totalCount }
                issues { totalCount }
                createdAt
                updatedAt
                pushedAt
                isPrivate
                isFork
                isArchived
                diskUsage
                homepageUrl
                licenseInfo {
                    name
                    spdxId
                }
            }
        }
    }
}
"#;

// ==================== HANDLERS ====================

// GET /api/v1/saludar?nombre=XXX
async fn saludar(req: HttpRequest, query: web::Query<QueryParams>) -> HttpResponse {
    #[derive(Serialize)]
    struct SaludoData {
        nombre: String,
        mensaje: String,
    }

    let data = SaludoData {
        nombre: query.nombre.clone(),
        mensaje: format!("¡Hola, {}!", query.nombre),
    };

    build_response(&req, data)
}

// GET /api/v1/github/users/{username}
async fn get_github_user(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let username = path.into_inner();

    #[derive(Serialize)]
    struct UserResponse {
        username: String,
        perfil: Option<GitHubUser>,
        mensaje: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        info_extendida: Option<bool>,
    }

    // Usar query extendida si hay token
    let query = if GITHUB_TOKEN.is_some() {
        QUERY_USER_EXTENDED
    } else {
        QUERY_USER_BASIC
    };

    #[derive(Deserialize)]
    struct UserData {
        user: GitHubUserGraphQL,
    }

    let variables = Some(serde_json::json!({ "username": username }));

    match graphql_query::<UserData>(query, variables).await {
        Ok(data) => {
            let user: GitHubUser = data.user.into();
            build_response(&req, UserResponse {
                username: username.clone(),
                perfil: Some(user),
                mensaje: format!("Usuario {} encontrado", username),
                info_extendida: Some(GITHUB_TOKEN.is_some()),
            })
        }
        Err(e) => {
            build_error_response(&req, actix_web::http::StatusCode::NOT_FOUND, UserResponse {
                username: username.clone(),
                perfil: None,
                mensaje: format!("Error: {}", e),
                info_extendida: None,
            })
        }
    }
}

// GET /api/v1/github/users/{username}/repos
async fn get_github_repos(req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let username = path.into_inner();

    #[derive(Serialize)]
    struct ReposResponse {
        username: String,
        total: usize,
        repositorios: Vec<GitHubRepo>,
    }

    #[derive(Deserialize)]
    struct ReposData {
        user: Repositories,
    }

    #[derive(Deserialize)]
    struct Repositories {
        repositories: RepoNodes,
    }

    #[derive(Deserialize)]
    struct RepoNodes {
        nodes: Vec<GitHubRepoGraphQL>,
    }

    let variables = Some(serde_json::json!({ 
        "username": username,
        "first": 100
    }));

    match graphql_query::<ReposData>(QUERY_USER_REPOS, variables).await {
        Ok(data) => {
            let repos: Vec<GitHubRepo> = data.user.repositories.nodes.into_iter().map(|r| r.into()).collect();
            build_response(&req, ReposResponse {
                username: username.clone(),
                total: repos.len(),
                repositorios: repos,
            })
        }
        Err(e) => {
            build_error_response(&req, actix_web::http::StatusCode::NOT_FOUND, ReposResponse {
                username: username.clone(),
                total: 0,
                repositorios: vec![],
            })
        }
    }
}

// GET /api/v1/github/status
async fn get_github_status(req: HttpRequest) -> HttpResponse {
    #[derive(Serialize)]
    struct StatusResponse {
        api: String,
        graphql_disponible: bool,
        token_configurado: bool,
        mensaje: String,
    }

    build_response(&req, StatusResponse {
        api: "GitHub GraphQL API".to_string(),
        graphql_disponible: true,
        token_configurado: GITHUB_TOKEN.is_some(),
        mensaje: if GITHUB_TOKEN.is_some() {
            "Token configurado - Información extendida disponible".to_string()
        } else {
            "Sin token - Solo información pública disponible".to_string()
        },
    })
}

// POST /api/v1/github/token (para verificar/actualizar token)
#[derive(Deserialize)]
struct TokenRequest {
    token: String,
}

#[derive(Serialize)]
struct TokenResponse {
    mensaje: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    validado: Option<bool>,
}

async fn set_github_token(req: HttpRequest, body: web::Json<TokenRequest>) -> HttpResponse {
    // Nota: El token pasado aquí no se persiste entre reinicios
    // Para uso persistente, usar variable de entorno GITHUB_TOKEN
    build_response(&req, TokenResponse {
        mensaje: "Para configurar el token de forma persistente, use la variable de entorno GITHUB_TOKEN".to_string(),
        validado: Some(!body.token.is_empty()),
    })
}

// ==================== MAIN ====================

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Cargar variables de entorno
    dotenv::dotenv().ok();

    println!("🚀 Servidor GraphQL corriendo en http://127.0.0.1:8080");
    println!("📚 Endpoints disponibles:");
    println!("   GET  /api/v1/saludar?nombre=XXX");
    println!("   GET  /api/v1/github/status");
    println!("   GET  /api/v1/github/users/{{username}}");
    println!("   GET  /api/v1/github/users/{{username}}/repos");
    println!("");
    
    if GITHUB_TOKEN.is_some() {
        println!("✅ Token GitHub configurado - Información extendida disponible");
    } else {
        println!("⚠️  Sin token GitHub - Configurar GITHUB_TOKEN en .env");
    }

    // Railway asigna el puerto dinámicamente via variable de entorno PORT
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("0.0.0.0:{}", port);
    println!("🚀 Escuchando en http://{}", bind_addr);

    HttpServer::new(|| {
        // CORS: permite peticiones desde cualquier origen (tu cPanel)
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .route("/api/v1/saludar", web::get().to(saludar))
            .route("/api/v1/github/status", web::get().to(get_github_status))
            .route("/api/v1/github/users/{username}", web::get().to(get_github_user))
            .route("/api/v1/github/users/{username}/repos", web::get().to(get_github_repos))
            .route("/api/v1/github/token", web::post().to(set_github_token))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
