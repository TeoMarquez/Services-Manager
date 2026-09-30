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

El perfil `api` exige la variable de entorno `SERVICES_MANAGER_API_TOKEN` (32 caracteres como mínimo). Generá un valor aleatorio localmente, por ejemplo con `openssl rand -hex 32`; no lo guardes en el repositorio.

Compose monta el checkout y conserva el registry/cache de Cargo y `target/` en volúmenes nombrados. La base local `data/` queda dentro del checkout montado y se excluye de Git. Las pruebas CLI usan un directorio temporal propio.

El servicio `dev` no ejecuta systemd. Las pruebas normales usan proveedores falsos/mock y no solicitan privilegios ni sockets del host. No se habilita acceso privilegiado en el perfil común.

El perfil `api` publica el puerto 3000 en loopback del host, exige token de 32+ caracteres y ejecuta `SystemdProvider`. El contenedor no tiene acceso al manager systemd del host: el perfil sirve para smoke test del servidor, no para discovery/control real. Para validar esas operaciones, ejecutar el binario en Linux con permisos acotados o usar una VM de integración dedicada. No montar sockets privilegiados en el perfil de pruebas. El servidor nativo escucha en `127.0.0.1:3000` por defecto.
