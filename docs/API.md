# API externa — estado actual

La implementación actual es REST/JSON bajo `/api/v1`. Es un primer contrato funcional; antes de ofrecer clientes externos estables aún deben cerrarse compatibilidad, límites, errores y seguridad operacional.

## Operaciones previstas

- `discover`: iniciar/consultar discovery progresivo y conocer si el ciclo está en curso, completo o falló.
- `search` / `list`: consultar por texto y filtros (presencia, visibilidad, tipo, tags), con paginación.
- `set visibility`: mostrar u ocultar metadata de visibilidad del gestor.
- `create service`: escribe una plantilla `.service` con nombre y descripción, recarga systemd y completa discovery.
- `service directory`: consultar/cambiar el directorio existente donde se escriben esos archivos.
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
| `GET` | `/api/v1/services/{unit_name}` | Devuelve ficha desde DB y `active` consultado en tiempo real a systemd |
| `POST` | `/api/v1/services` | Crea plantilla; cuerpo `{"name":"worker","description":"Worker de ejemplo"}` |
| `GET`, `PUT` | `/api/v1/settings/service-directory` | Obtiene/cambia la carpeta absoluta existente; PUT recibe `{"path":"/etc/systemd/system"}` |
| `PUT` | `/api/v1/services/{unit_name}/visibility` | Implementado, metadata del gestor |
| `PUT` | `/api/v1/services/{unit_name}/alias` | Body `{"alias":"nuevo-alias"}`; guarda el alias y responde `204`; también admite ID numérico como identificador |
| `PUT` | `/api/v1/services/{unit_name}/description` | Body `{"description":"nueva descripción"}`; persiste en DB y responde `204` |
| `POST` | `/api/v1/services/{unit_name}/start` | Implementado mediante coordinador OS → verificación → DB → compensación |
| `POST` | `/api/v1/services/{unit_name}/stop` | Implementado mediante coordinador OS → verificación → DB → compensación |
| `PUT` | `/api/v1/services/{unit_name}/startup-mode` | Implementado; cuerpo `{"mode":"enabled"}` o `{"mode":"disabled"}` |
| `GET`, `POST` | `/api/v1/tags` | Implementado |
| `DELETE` | `/api/v1/tags/{tag_name}` | Implementado |
| `GET` | `/api/v1/services/{unit_name}/tags` | Implementado |
| `PUT`, `DELETE` | `/api/v1/services/{unit_name}/tags/{tag_name}` | Implementado |
| `POST` | `/api/v1/reset` | Implementado, elimina solo datos gestionados y checkpoints |

El binario `api` usa `SystemdProvider`; sus rutas de discovery/control invocan `systemctl`. Los estados admitidos son `active`/`inactive` y `enabled`/`disabled`; `failed`, estados transitorios y estados de inicio como `masked`, `static` o `enabled-runtime` se rechazan. La CLI continúa utilizando el mock. Tests con proveedor falso verifican la ruta `start`; tests de integración con un host systemd siguen pendientes.

El detalle de servicio consulta `systemctl show --property=ActiveState --value -- {unit_name}` en cada petición y añade `active: true` para `ActiveState=active` o `active: false` para `inactive`. El valor es una observación en vivo, no el estado guardado en la DB. Si systemd devuelve un estado transitorio/no soportado o falla la consulta, la API responde `502`; una unidad que no está en el inventario de la DB devuelve `404`.

La plantilla creada contiene `[Unit]`, la descripción y una sección `[Service]` de tipo `oneshot`, sin `ExecStart`. Debe completarse antes de ejecutar el servicio. La ruta de creación escribe exclusivamente `{name}.service`; rechaza nombres con separadores/ruta o caracteres fuera de la lista permitida. Tras escribir, solicita `systemctl daemon-reload` y corre un discovery completo. Si systemd no reconoce el archivo por estar en una carpeta ajena a su unit search path, el endpoint devuelve error y deja el archivo para revisión.

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

El binario lee primero `SERVICES_MANAGER_API_TOKEN` desde `.env`. Si no hay token guardado, usa la variable de entorno si está configurada o genera uno criptográficamente aleatorio, lo guarda en `.env` y lo imprime al iniciar. `.env` queda ignorado por Git y en Unix se escribe con permisos `0600`.

Opciones de ejecución: `--token VALOR` usa/guarda un token inicial; `--token ""` genera uno si no hay uno persistido; si `.env` ya tiene otro secreto, se requiere `--overwrite-token`. Ese flag guarda el token dado por `--token` o genera uno nuevo si no se pasó valor. El token se imprime al iniciar, salvo que se pase `--silent-token`. Por ejemplo, `./run.sh --overwrite-token --silent-token` rota el token y lo deja en `.env` sin imprimirlo.

Un token suministrado por argumento puede quedar visible en historial del shell o lista de procesos. Para pruebas, es preferible dejar que se genere y se guarde; para configurar uno manualmente, proteger el historial y los permisos del usuario.

El binario escucha en `127.0.0.1:3000` por defecto; `SERVICES_MANAGER_API_BIND` puede cambiarlo. Tras abrir correctamente el socket, imprime el puerto efectivo. La autenticación es un único token compartido, sin roles por operación. Compose publica el puerto en loopback del host, pero el contenedor no tiene acceso al systemd del host: sus rutas de discovery/control no validan el proveedor real.

`--port PORT` selecciona un puerto entre 1 y 65535, lo guarda como `SERVICES_MANAGER_API_PORT` en `.env` y lo usa en el arranque actual; sin el argumento se reutiliza el puerto guardado o se usa `3000`. `SERVICES_MANAGER_API_PORT` del entorno tiene precedencia sobre `.env`; `SERVICES_MANAGER_API_BIND` puede elegir la IP de escucha y su puerto se usa como fallback al inicializar. El proceso imprime el puerto efectivo al iniciar. Los scripts `./run.sh --port 8080` y `run.bat --port 8080` reenvían el argumento.

`.env`, `migrations/` y `data/` se mantienen relativos a la raíz del proyecto. Los scripts `run.bat` y `run.sh` cambian a esa raíz antes de lanzar el binario y reenvían los argumentos recibidos. El propio binario también busca la carpeta `migrations/` en los directorios antecesores de su ejecutable y usa esa raíz como directorio actual. Mantener `migrations/` junto a la raíz de despliegue y dar permisos de escritura al usuario sobre `.env` y `data/`.

## Contratos aún necesarios

Pendiente antes de considerar estable la API: versionado/compatibilidad, esquema de error consistente, límites de peticiones/búsqueda, timeout/cancelación, permisos/autorización operacional, concurrencia/idempotencia, estado consultable del discovery y revisión del cursor potencialmente grande. Validar crear y recargar unidades en el Linux objetivo.
