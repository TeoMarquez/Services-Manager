# Estrategia de pruebas

## Estado ejecutable

- `cargo test --offline`: 18 tests unitarios y 1 test CLI pasan en Windows.
- `docker compose run --rm dev cargo test --locked`: la misma suite pasa en el contenedor Linux.
- Los tests de control usan proveedor falso; no han ejercitado `systemctl` real.
- Ya están cubiertos discovery por páginas/reanudación, búsqueda/paginación, migración idempotente, acceso bearer, endpoints de lectura/reset/discovery/start y control éxito/error OS/fallo DB con rollback exitoso.

## Capas

### Unitarias

- Reglas de descubrimiento, identidad, mapeo y transición.
- Construcción de filtros, parámetros, paginación y resultados vacíos.
- Tags: creación idempotente, asociación duplicada, eliminación y cascada.
- Control coordinado: orden OS → verificación → DB y compensación ante fallo.
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
