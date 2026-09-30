# Estado actual

Última revisión: 2026-09-30.

## Base técnica

- Aplicación binaria Rust 2024 (`Services-Manager`), con SQLite mediante `rusqlite` y SQLite bundled.
- La conexión crea `data/`, abre `data/services.db`, activa claves foráneas y ejecuta migraciones SQL desde `migrations/`.
- Migraciones versionadas 01–05: esquema inicial, presencia, checkpoint durable de discovery, estado operacional observado y configuración de carpeta de servicios (por defecto `/etc/systemd/system`).
- Repositorios para servicios y tags; relaciones con eliminación en cascada.
- Proveedor abstracto `SystemProvider`; `MockSystem` lee `mock/systemd.json`; `SystemdProvider` consulta/controla systemd por `systemctl`.
- CLI interactiva para sincronización, búsqueda/listado, visibilidad, eliminación y operaciones de tags.

## Comportamiento comprobado leyendo el código

Al iniciar, el binario CLI crea la DB y repositorios, ejecuta `services::sync::sync` con `MockSystem` y después abre el menú. El binario API usa `SystemdProvider`. Discovery procesa páginas, persiste sus registros y checkpoint de cursor en una transacción por página, y reanuda tras error. Solo marca ausencias al guardar la última página del ciclo. Systemd guarda el snapshot de unit files en su cursor para mantener estable el inventario lógico de ese ciclo.

La capa de consultas ofrece filtros combinables/paginación y búsqueda parametrizada por nombre, alias o descripción. La API REST implementa discovery incremental, consultas, alias, tags, visibilidad, reset, control operacional, creación de plantillas `.service` y configuración persistente del directorio. La CLI continúa en mock.

El mock solo provee nombres de unidad y no muta el sistema. `SystemdProvider` enumera unit files y permite observar/iniciar/detener/habilitar/deshabilitar servicios y ejecutar daemon-reload. La API crea archivos `.service` sin sobrescribir, como plantillas sin `ExecStart`, luego recarga systemd y ejecuta discovery completo. La plantilla requiere editar `ExecStart` antes de poder iniciar una tarea real. El coordinador controla el orden OS→DB. Los estados no representados se rechazan. Las llamadas a `systemctl` todavía no tienen timeout configurado y la integración real de unit-file creation no se ha validado en el host Linux.

## Pruebas y entorno

- Tests unitarios inline para token/.env, API, creación de plantilla y discovery, checkpoint/reanudación, filtros, paginación, migraciones y handshake/compensación; test CLI por subprocess en un directorio temporal. Tags no tiene tests propios todavía.
- Antes de los cambios de alias/puerto, la suite de 27 tests unitarios y 1 CLI pasó en Windows y Docker Linux. Tras esos cambios, `cargo fmt --all -- --check` y `cargo check --locked --all-targets` pasan; falta reejecutar tests.
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
- Estado anterior: 19 tests pasaban antes del token y creación de servicios. Estado actual: `cargo test --offline` y `docker compose run --rm dev cargo test --locked` pasan con 27 unitarios + 1 CLI.
- El Dockerfile compila la aplicación real y Compose usa un volumen target nuevo; se evitó inicializar el volumen con un binario stub.
- `cargo fmt -- --check` global aún reporta formato heredado en módulos no reformateados; se formatearon los módulos tocados directamente sin reformatear el árbol completo.
- Se retiraron `chrono` y `refinery`, que no tenían uso directo; `refinery` resolvía `time 0.3.55`, incompatible con Rust 1.87 del entorno. `Cargo.lock` se regeneró offline y los tests compilaron con el toolchain disponible.
- El mock parsea el JSON completo en cada página. `SystemdProvider` reenumera unit files al comenzar cada ciclo y almacena la lista snapshot en el cursor durable: es estable, pero el cursor crece proporcionalmente al inventario.

La DB representa el estado observado del sistema. Para control de servicios, se ejecuta primero la operación en el sistema operativo; solo tras confirmación se persiste el estado en DB. La API puede crear plantillas, configurar su carpeta, y fuerza discovery después. La plantilla no ejecuta un proceso porque no incluye `ExecStart`. La CLI sigue en mock y no ofrece todavía start/stop/modo de inicio. El contenedor normal valida build y tests; no accede al systemd del host.

El token de API vive en `.env` en la raíz detectada del despliegue, que queda ignorado por Git. Al faltar, se genera y guarda; `--overwrite-token` rota/establece el secreto. Se imprime al iniciar, salvo que se pase `--silent-token`. `--port` selecciona y persiste `SERVICES_MANAGER_API_PORT`; el puerto efectivo se imprime al abrir el socket. La variable de entorno del token se usa como fallback si `.env` no tiene token. `run.bat` y `run.sh` arrancan desde la raíz del repositorio y reenvían sus argumentos; el binario también la detecta a partir de `migrations/` en sus directorios antecesores.
