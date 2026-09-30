# Desarrollo y pruebas con Docker

El servicio `dev` usa Rust 1.88 sobre Debian Bookworm, compila SQLite bundled y no solicita privilegios ni sockets del host. El mismo Compose sirve en Windows y Linux con Docker Desktop/Engine.

Desde la raíz del repositorio:

```sh
docker compose build dev
docker compose run --rm dev cargo test --locked
docker compose run --rm dev cargo check --locked
docker compose run --rm dev cargo run
docker compose --profile api up --build api
```

El perfil `api` carga el token de `.env` o lo genera/guarda si falta; también acepta `SERVICES_MANAGER_API_TOKEN` como fallback. Para rotarlo desde un contenedor de desarrollo se puede ejecutar `docker compose run --rm dev cargo run --locked --bin api -- --overwrite-token --silent-token`. El token requiere al menos 32 bytes y `.env` está ignorado por Git.

Compose monta el checkout y conserva el registry/cache de Cargo y `target/` en volúmenes nombrados. La base local `data/` queda dentro del checkout montado y se excluye de Git. Las pruebas CLI usan un directorio temporal propio.

El servicio `dev` no ejecuta systemd. Las pruebas normales usan proveedores falsos/mock y no solicitan privilegios ni sockets del host. No se habilita acceso privilegiado en el perfil común.

El perfil `api` publica en loopback el puerto `SERVICES_MANAGER_API_PORT` de `.env` (3000 por defecto), ejecuta `SystemdProvider` y usa `.env` persistido en el checkout montado. El contenedor no tiene acceso al manager systemd del host: el perfil sirve para smoke test del servidor, no para discovery/control/daemon-reload real. Para validar esas operaciones, ejecutar el binario en Linux con permisos acotados o usar una VM de integración dedicada. No montar sockets privilegiados en el perfil de pruebas. El servidor nativo escucha en `127.0.0.1:3000` por defecto; `./run.sh --port 8080` cambia y persiste el puerto.
