# Estrategia de pruebas

## Estado ejecutable

- La última ejecución registrada pasó 28 tests unitarios y 1 test CLI con `cargo test --locked` en Windows.
- Tras los cambios de alias/puerto, `cargo fmt --all -- --check` y `cargo check --locked --all-targets` pasan; la suite no se ha vuelto a ejecutar.
- Los tests de control usan proveedor falso; no han ejercitado `systemctl` real.
- La suite definida cubre discovery por páginas/reanudación, búsqueda/paginación, migración idempotente, token/.env, acceso bearer, endpoints de lectura/reset/discovery/start/configuración/creación de unit file y control éxito/error OS/fallo DB con rollback exitoso.
- No hay tests para operaciones CRUD de tags, asignación de alias, detalle de servicio/estado activo o selección/persistencia de puerto. La ruta de descripción ya verifica respuesta y persistencia.

## Ubicación y organización

- Los tests unitarios están junto al código, dentro de módulos `#[cfg(test)]`: `src/api.rs`, `src/token.rs`, `src/db/migrations.rs`, `src/db/settings.rs`, `src/services/control.rs`, `src/services/create.rs`, `src/services/list.rs` y `src/services/sync.rs`.
- El test de CLI de proceso completo vive en `tests/cli.rs`; crea una carpeta temporal con migraciones y fixture mock y conduce el menú mediante stdin.
- Esta organización facilita probar detalles internos de cada componente. La suite API está concentrada en `src/api.rs`, por lo que conviene separar tests de rutas al crecer, y faltan tests de repositorio/tags y nuevas rutas.
- Ejecutar todos con `cargo test`; ejecutar el test de integración CLI con `cargo test --test cli`.

## Capas

### Unitarias

- Reglas de descubrimiento, identidad, mapeo y transición.
- Construcción de filtros, parámetros, paginación y resultados vacíos.
- Tags: aún sin tests automatizados de repositorio/API.
- Control coordinado: orden OS → verificación → DB y compensación ante fallo.
- Token: generación, persistencia, prioridad de `.env` y rotación explícita.
- Puerto: agregar pruebas de precedencia CLI/entorno/`.env`, validación de rango y persistencia preservando el token y otras variables.
- Alias/detalle: agregar pruebas de API por nombre e ID, alias inválido/servicio inexistente y búsqueda posterior; probar detalle con servicio activo/inactivo, servicio ausente y error del proveedor.
- Creación: validación del nombre/descripcion, escritura de plantilla, rollback tras fallo de daemon-reload y discovery posterior usando proveedor falso.
- Pendiente: verificación que falla/desacuerda, operación OS parcialmente aplicada, rollback fallido, indeterminación, llamadas concurrentes/idempotencia y timeout.

### Integración con SQLite

- Crear DB temporal, aplicar todas las migraciones y comprobar esquema/constraints.
- Repositorios de servicios, presencia y tags.
- Reconciliar un ciclo completo, un ciclo parcial fallido y ciclos repetidos.
- Confirmar que fallos no dejan escrituras parciales de una operación DB.

### CLI

- Extraer lógica de argumentos/entrada para invocarla de forma automatizable.
- Probar comandos válidos, argumentos faltantes/invalidos, servicio/tag inexistente y errores del proveedor/DB.
- Probar que los mensajes diferencian fallo controlado, rollback exitoso y divergencia tras rollback fallido.
- Preferir subcomandos no interactivos para automatización; conservar menú interactivo solo si sigue siendo requisito.

### API

- Pruebas de contrato de rutas, cuerpos, validaciones, códigos de respuesta y errores.
- Flujos end-to-end sobre DB temporal/proveedor falso.
- Verificar auth/permisos, timeouts, repetición de solicitudes y aislamiento de operaciones.

## Matriz mínima de fallos de control

| Caso | OS | Verificación | DB | Compensación | Resultado esperado |
|---|---|---|---|---|---|
| Éxito | aplica | coincide | guarda | no | éxito |
| Falla OS sin efecto | falla | estado previo | intacta | no necesaria | error OS |
| Falla DB | aplica | coincide | falla | revierte y verifica | error DB, estado previo restaurado |
| Falla DB y rollback | aplica | coincide | falla | falla/no verifica | error compuesto y divergencia visible |
| Interrupción/timeout | incierto | consulta adicional | no asumir | según estado | confirmado, revertido o indeterminado |

## Entorno reproducible y faltantes

El perfil `dev` corre la suite mock/fake en Docker sin privilegios ni sockets del host. El perfil `api` puede compilar y arrancar el servidor, pero no acceder a systemd porque no comparte el manager del host. Preparar una prueba de integración optativa en VM/host Linux con un conjunto controlado de unidades y permisos explícitos.
