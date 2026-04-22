# ========================
# ETAPA 1: Build
# ========================
FROM rust:1.75-slim AS builder

# Instalar dependencias del sistema
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiar archivos de dependencias primero (cache de capas)
COPY Cargo.toml Cargo.lock ./

# Crear src dummy para compilar dependencias (optimización de caché)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copiar código real y compilar
COPY src ./src
RUN touch src/main.rs && cargo build --release

# ========================
# ETAPA 2: Runtime
# ========================
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiar binario compilado
COPY --from=builder /app/target/release/rust_api .

# Puerto que expone la app
EXPOSE 8080

# Variable de entorno para que Railway sepa el puerto
ENV PORT=8080

CMD ["./rust_api"]
