# Estado actual

Última revisión: 2026-09-29.

## Base técnica

- Aplicación binaria Rust 2024 (`Services-Manager`), con SQLite mediante `rusqlite` y SQLite bundled.
- La conexión crea `data/`, abre `data/services.db`, activa claves foráneas y ejecuta migraciones SQL desde `migrations/`.
- Migraciones versionadas 01–04: esquema inicial, presencia, checkpoint durable de discovery y tabla de estado operacional observado.
- Repositorios para servicios y tags; relaciones con eliminación en cascada.
- Proveedor abstracto `SystemProvider`; `MockSystem` lee `mock/systemd.json`; `SystemdProvider` consulta/controla systemd por `systemctl`.
- CLI interactiva para sincronización, búsqueda/listado, visibilidad, eliminación y operaciones de tags.

## Comportamiento comprobado leyendo el código

Al iniciar, el binario CLI crea la DB y repositorios, ejecuta `services::sync::sync` con `MockSystem` y después abre el menú. El binario API usa `SystemdProvider`. Discovery procesa páginas, persiste sus registros y checkpoint de cursor en una transacción por página, y reanuda tras error. Solo marca ausencias al guardar la última página del ciclo. Systemd guarda el snapshot de unit files en su cursor para mantener estable el inventario lógico de ese ciclo.

La capa de consultas ofrece filtros combinables/paginación y búsqueda parametrizada por nombre, alias o descripción. La API REST implementa discovery incremental, consultas, tags, visibilidad, reset y control operacional. La CLI continúa en mock.

El mock solo provee nombres de unidad y no muta el sistema. `SystemdProvider` enumera unit files y permite observar/iniciar/detener/habilitar/deshabilitar servicios. El coordinador obtiene estado previo, aplica OS, verifica y persiste; ante error intenta restaurar y verificar el estado previo. Los estados admitidos son `active`/`inactive` y `enabled`/`disabled`; `failed`, estados transitorios y `masked`, `static`, `enabled-runtime`, etc. se rechazan para evitar reducirlos a un booleano. Las llamadas a `systemctl` todavía no tienen timeout configurado y no han sido validadas contra un manager systemd real.

## Pruebas y entorno

- Tests unitarios de discovery, checkpoint/reanudación, filtros, paginación, tags y handshake/compensación; test CLI por subprocess en un directorio temporal.
- Docker Compose validado con Rust 1.88; la suite completa pasó en Windows y dentro del contenedor Linux.
- Las migraciones se ejecutan mediante código propio. `chrono` y `refinery` no tenían uso directo, así que se retiraron del manifiesto para evitar dependencias innecesarias; `thiserror` se usa para propagar errores de proveedor/discovery.

## Trabajo acordado, aún pendiente

1. Validar proveedor `systemctl` en Linux real/VM: argumentos, parsing, permisos y comportamiento al desaparecer una unidad; implementar timeouts.
2. Ampliar pruebas del coordinador para fallo de verificación, fallo de rollback, estado indeterminado, acción OS parcialmente aplicada e idempotencia; añadir tests API para stop y startup-mode.
3. Compartir casos de uso entre CLI/API; definir autorización más allá del token bearer compartido, concurrencia y semántica/reintentos de reset/discovery.

## Avance de implementación

- El proveedor ahora devuelve errores tipados de lectura/parseo en vez de hacer panic.
- Discovery expone `sync_step` para procesar una página y `sync` para completar el ciclo; estado y cursor se guardan en SQLite para reanudar tras errores.
- Las observaciones por página y el cursor avanzan dentro de la misma transacción; la reconciliación de ausencias y el cierre del ciclo ocurren juntos al final.
- Se agregaron tests de lotes, reanudación tras fallo, no marcar ausencias antes de completar, idempotencia, búsqueda literal, filtro tag y paginación.
- Test CLI de subprocess en workspace temporal prueba búsqueda y listado sin tocar `data/` del desarrollador.
- Estado previo al control: 15 tests (14 unitarios + 1 CLI) pasaban en Windows y Docker Linux. Estado actual: `cargo test --offline` y `docker compose run --rm dev cargo test --locked` pasan con 19 tests (18 unitarios + 1 CLI).
- El Dockerfile compila la aplicación real y Compose usa un volumen target nuevo; se evitó inicializar el volumen con un binario stub.
- `cargo fmt -- --check` global aún reporta formato heredado en módulos no reformateados; se formatearon los módulos tocados directamente sin reformatear el árbol completo.
- Se retiraron `chrono` y `refinery`, que no tenían uso directo; `refinery` resolvía `time 0.3.55`, incompatible con Rust 1.87 del entorno. `Cargo.lock` se regeneró offline y los tests compilaron con el toolchain disponible.
- El mock parsea el JSON completo en cada página. `SystemdProvider` reenumera unit files al comenzar cada ciclo y almacena la lista snapshot en el cursor durable: es estable, pero el cursor crece proporcionalmente al inventario.

La DB representa el estado observado del sistema. Para control de servicios, se ejecuta primero la operación en el sistema operativo; solo tras confirmación se persiste el estado en DB. Ante error se intenta compensar y verificar. La API usa el proveedor systemd; la CLI sigue en mock y no ofrece todavía start/stop/modo de inicio. El contenedor normal valida build y tests; no ejecuta ni accede al systemd del host.
