# API externa — estado actual

La implementación actual es REST/JSON bajo `/api/v1`. Es un primer contrato funcional; antes de ofrecer clientes externos estables aún deben cerrarse compatibilidad, límites, errores y seguridad operacional.

## Operaciones previstas

- `discover`: iniciar/consultar discovery progresivo y conocer si el ciclo está en curso, completo o falló.
- `search` / `list`: consultar por texto y filtros (presencia, visibilidad, tipo, tags), con paginación.
- `set visibility`: mostrar u ocultar metadata de visibilidad del gestor.
- `reset`: reiniciar los datos gestionados por Services Manager; no modifica servicios del sistema operativo.
- `start`, `stop`: controlar el estado operacional del servicio.
- `set startup mode`: cambiar entre `enabled` y `disabled` (los demás modos systemd todavía no están soportados).
- `tags`: crear, listar, asignar, quitar y consultar etiquetas.

## Rutas de la primera iteración

| Método | Ruta | Estado |
|---|---|---|
| `GET` | `/api/v1/health` | Implementado |
| `POST` | `/api/v1/discovery` | Implementado, procesa una página (`batch_size` opcional) |
| `GET` | `/api/v1/services` | Implementado: `search`, `present`, `visible`, `system_service`, `tag_id`, `page`, `per_page` |
| `PUT` | `/api/v1/services/{unit_name}/visibility` | Implementado, metadata del gestor |
| `POST` | `/api/v1/services/{unit_name}/start` | Implementado mediante coordinador OS → verificación → DB → compensación |
| `POST` | `/api/v1/services/{unit_name}/stop` | Implementado mediante coordinador OS → verificación → DB → compensación |
| `PUT` | `/api/v1/services/{unit_name}/startup-mode` | Implementado; cuerpo `{"mode":"enabled"}` o `{"mode":"disabled"}` |
| `GET`, `POST` | `/api/v1/tags` | Implementado |
| `DELETE` | `/api/v1/tags/{tag_name}` | Implementado |
| `GET` | `/api/v1/services/{unit_name}/tags` | Implementado |
| `PUT`, `DELETE` | `/api/v1/services/{unit_name}/tags/{tag_name}` | Implementado |
| `POST` | `/api/v1/reset` | Implementado, elimina solo datos gestionados y checkpoints |

El binario `api` usa `SystemdProvider`; sus rutas de discovery/control invocan `systemctl`. Los estados admitidos son `active`/`inactive` y `enabled`/`disabled`; `failed`, estados transitorios y estados de inicio como `masked`, `static` o `enabled-runtime` se rechazan. La CLI continúa utilizando el mock. Tests con proveedor falso verifican la ruta `start`; tests de integración con un host systemd siguen pendientes.

## Reglas del contrato

- Las mutaciones operacionales llaman al coordinador OS → verificación → DB → compensación; no ejecutan SQL directamente desde la capa de transporte.
- Distinguir errores de validación, recurso inexistente, permisos, timeout, proveedor, persistencia, compensación y estado indeterminado. Hay errores tipados iniciales, pero su esquema externo aún no está estabilizado.
- El resultado de control incluye estado deseado, estado observado y resultado de compensación cuando aplique.
- Discovery debe aceptar/reconocer repetición y exponer estado del ciclo; no marcar ausencias con un ciclo incompleto.
- Búsqueda recibe parámetros tipados; no aceptar fragmentos SQL del cliente.
- Las llamadas `systemctl` aún carecen de timeout/cancelación; agregarlos y probar solicitudes repetidas/concurrentes.
- Proteger la API con límites de exposición y autorización antes de habilitar control del sistema.
- El binario servidor requiere `SERVICES_MANAGER_API_TOKEN` de al menos 32 caracteres y valida `Authorization: Bearer <token>` para todas las rutas.

## Ejecución y seguridad actual

El binario requiere `SERVICES_MANAGER_API_TOKEN` de al menos 32 caracteres y exige `Authorization: Bearer <token>` en todas las rutas. Por defecto escucha en `127.0.0.1:3000`; `SERVICES_MANAGER_API_BIND` puede cambiarlo. La autenticación actual es un token compartido, sin roles/autorización por operación. Compose publica el puerto en loopback del host, pero el contenedor no tiene acceso al systemd del host: sus rutas de discovery/control no validan el proveedor real.

## Contratos aún necesarios

Pendiente antes de considerar estable la API: versionado/compatibilidad, esquema de error consistente, límites de peticiones/búsqueda, timeout/cancelación, autorización por operación, concurrencia/idempotencia, estado consultable del discovery y revisión del cursor potencialmente grande. Reset ya elimina únicamente datos gestionados por Services Manager; confirmar si se requiere un mecanismo explícito de confirmación externa.
