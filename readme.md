# Services Manager

**Gestiona y consulta servicios de Linux mediante una API REST**, con SQLite para guardar inventario y metadata, y `systemd` para descubrir y controlar las unidades reales.

La API permite a un bot, script o aplicación externa descubrir servicios, buscarlos, organizarlos con tags, actualizar alias y descripciones, y consultar o cambiar su estado. El sistema operativo es la fuente de verdad: para iniciar o detener una unidad, primero se realiza y verifica la operación en systemd y luego se actualiza la base de datos.

> La integración real está pensada para Linux con systemd. En Docker se pueden ejecutar las pruebas, pero el contenedor de desarrollo no controla el systemd del host.

## Qué puedes hacer

- Descubrir unidades systemd progresivamente y reanudar el ciclo si el proceso se interrumpe.
- Buscar y filtrar servicios por nombre, descripción, alias, presencia, visibilidad y tags.
- Consultar el estado activo directamente desde systemd.
- Iniciar, detener y habilitar o deshabilitar servicios.
- Organizar servicios con tags y controlar su visibilidad en el gestor.
- Crear una plantilla `.service`, recargar systemd y descubrir la nueva unidad.
- Administrar la API con un token bearer y usar SQLite como almacenamiento persistente.

## Inicio rápido en Linux

Necesitas Rust/Cargo para compilar. Desde la carpeta del proyecto:

```sh
./build.sh
./run.sh
```

Al primer inicio se genera un token y se guarda en `.env`; por defecto se muestra en pantalla. La API escucha en `http://127.0.0.1:3000`. La base de datos se crea en `data/services.db`.

Para que el token no aparezca en la salida del proceso:

```sh
./run.sh --silent-token
```

El token sigue guardado en `.env`. Para configurar o rotar el token explícitamente:

```sh
./run.sh --token 'un-token-seguro-de-al-menos-32-caracteres' --overwrite-token
```

Para cambiar el puerto actual y guardarlo para próximos inicios:

```sh
./run.sh --port 8080
```

No publiques la API en una interfaz de red externa sin configurar conscientemente el bind, proteger el acceso y limitar quién puede alcanzar el proceso. El bind predeterminado es loopback.

## Probar la API

Todas las rutas requieren `Authorization: Bearer <token>`. Puedes leer el token guardado desde la raíz del repositorio:

```sh
TOKEN=$(sed -n 's/^SERVICES_MANAGER_API_TOKEN=//p' .env)
API=http://127.0.0.1:3000/api/v1
```

Consultar salud y buscar servicios:

```sh
curl -H "Authorization: Bearer $TOKEN" "$API/health"
curl -H "Authorization: Bearer $TOKEN" "$API/services?search=worker&page=1&per_page=25"
```

Procesar una página de discovery y consultar el detalle/estado vivo de una unidad:

```sh
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{}' "$API/discovery"
curl -H "Authorization: Bearer $TOKEN" "$API/services/worker.service"
```

En el detalle, `active` se obtiene de systemd al momento de la petición. Alias y descripción se pueden actualizar así:

```sh
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"alias":"worker principal"}' \
  "$API/services/worker.service/alias"
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"description":"Procesa tareas en segundo plano"}' \
  "$API/services/worker.service/description"
```

Ambas actualizaciones responden `204 No Content`. La lista completa de rutas y atributos está en [docs/API.md](docs/API.md).

## Instalación y permisos

La API consulta y controla systemd con `systemctl`. Para crear archivos bajo `/etc/systemd/system` y ejecutar `systemctl daemon-reload`, el proceso necesita los permisos correspondientes. Si se ejecuta como servicio systemd, configura usuario, permisos, reinicio y manejo del token para tu instalación.

La ruta del directorio de plantillas se configura mediante la API y por defecto es `/etc/systemd/system`. Las plantillas creadas incluyen nombre y descripción, **pero no `ExecStart`**: complétalo antes de intentar iniciar la unidad. La guía de operación y las restricciones del contrato están en [docs/API.md](docs/API.md) y [docs/SYSTEM_CONTROL.md](docs/SYSTEM_CONTROL.md).

## Desarrollo y pruebas

Los scripts `build.sh`/`run.sh` son para Linux y `build.bat`/`run.bat` para Windows. También puedes usar Docker para un entorno de desarrollo reproducible:

```sh
docker compose build dev
docker compose run --rm dev cargo test --locked
docker compose run --rm dev cargo check --locked
```

El servicio `dev` usa proveedores de prueba y no requiere privilegios. El perfil Docker de API tampoco comparte el manager systemd del host; valida el servidor, no el control real de unidades.

## Documentación

- [Contrato y rutas de API](docs/API.md)
- [Discovery y reanudación](docs/DISCOVERY.md)
- [Control systemd y consistencia](docs/SYSTEM_CONTROL.md)
- [Pruebas](docs/TESTING.md)
- [Entorno Docker](docs/DOCKER.md)
- [Decisiones de arquitectura](docs/ARCHITECTURE_DECISIONS.md)
