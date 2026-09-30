# Services Manager

**REST API para descubrir, consultar y controlar servicios de Linux mediante `systemd`.**

Services Manager centraliza el inventario y la administración de unidades `systemd` detrás de una API. Permite descubrir servicios, buscarlos, organizarlos con tags, administrar metadata y controlar su estado desde un bot, script o aplicación externa.

La metadata administrada por el gestor se persiste en SQLite, mientras que **`systemd` sigue siendo la fuente de verdad para el estado real de las unidades**.

> Diseñado para Linux con `systemd`. Docker permite ejecutar el servidor y las pruebas, pero no controla el `systemd` del host.

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-passing-brightgreen?style=for-the-badge)](#desarrollo)
[![systemd](https://img.shields.io/badge/systemd-supported-blue?style=for-the-badge&logo=linux&logoColor=white)](#)

</div>

---

## ¿Qué resuelve?

Administrar servicios directamente con `systemctl` funciona bien cuando hay pocos servicios y se trabaja manualmente. Cuando otra aplicación necesita descubrirlos, buscarlos y operar sobre ellos, empieza a ser útil una capa intermedia.

Services Manager proporciona esa capa:

```text
             Bot / Script / Aplicación
                       │
                       │ HTTP / REST
                       ▼
              ┌──────────────────┐
              │  Services Manager│
              │       API        │
              └────────┬─────────┘
                       │
              ┌────────┴─────────┐
              │                  │
              ▼                  ▼
       ┌─────────────┐    ┌─────────────┐
       │   SQLite    │    │   systemd   │
       │  Metadata   │    │ Estado real │
       └─────────────┘    └─────────────┘
```

SQLite almacena información administrada por Services Manager, como aliases, descripciones, tags y visibilidad.

El estado operativo se consulta directamente desde `systemd`, evitando que una copia persistida quede desactualizada.

---

## Características

* 🔎 **Discovery progresivo** de unidades `systemd`, con posibilidad de reanudar el proceso.
* 🔍 **Búsqueda y filtrado** por nombre, descripción, alias, presencia, visibilidad y tags.
* ⚙️ **Estado en tiempo real** consultado directamente desde `systemd`.
* ▶️ **Control de servicios**: iniciar, detener, habilitar y deshabilitar unidades.
* 🏷️ **Tags** para organizar y clasificar servicios.
* 👁️ **Visibilidad** para controlar qué unidades aparecen en el gestor.
* 📝 **Metadata administrable**, incluyendo aliases y descripciones.
* 🧩 **Creación de plantillas `.service`** y recarga de `systemd`.
* 🔐 **Autenticación Bearer Token** para proteger la API.
* 💾 **SQLite** como almacenamiento persistente.
* 🐳 **Entorno Docker** para desarrollo y pruebas reproducibles.

---

## Ejemplo

Una aplicación puede consultar un servicio sin interactuar directamente con `systemctl`:

```http
GET /api/v1/services/worker.service
Authorization: Bearer <token>
```

Y recibir información combinada de la metadata almacenada y del estado actual de `systemd`:

```json
{
  "name": "worker.service",
  "alias": "Worker principal",
  "description": "Procesa tareas en segundo plano",
  "active": true,
  "visible": true,
  "tags": [
    "worker",
    "backend"
  ]
}
```

Si la aplicación necesita cambiar el estado:

```http
POST /api/v1/services/worker.service/start
Authorization: Bearer <token>
```

La operación se realiza sobre `systemd` y se verifica su resultado antes de actualizar la información correspondiente en Services Manager.

---

## Inicio rápido

### Requisitos

Para ejecutar la integración real necesitas:

* Linux
* `systemd`
* Rust / Cargo
* permisos suficientes para las operaciones que quieras realizar

Desde la carpeta del proyecto:

```sh
./build.sh
./run.sh
```

Al primer inicio se genera un token y se guarda en `.env`. Por defecto también se muestra en pantalla.

La API queda disponible en:

```text
http://127.0.0.1:3000
```

y la base de datos se crea en:

```text
data/services.db
```

### Token

Para iniciar el servidor sin mostrar el token en la salida:

```sh
./run.sh --silent-token
```

Para configurar o rotar explícitamente el token:

```sh
./run.sh --token 'un-token-seguro-de-al-menos-32-caracteres' --overwrite-token
```

### Puerto

Para cambiar el puerto y conservarlo para próximos inicios:

```sh
./run.sh --port 8080
```

El bind predeterminado es `127.0.0.1`.

> Si expones la API en una interfaz de red externa, configura conscientemente el bind, protege el acceso y limita qué dispositivos pueden alcanzar el proceso.

---

## API

Todas las rutas requieren:

```http
Authorization: Bearer <token>
```

Por ejemplo:

```sh
TOKEN=$(sed -n 's/^SERVICES_MANAGER_API_TOKEN=//p' .env)
API=http://127.0.0.1:3000/api/v1
```

Consultar el estado de la API:

```sh
curl \
  -H "Authorization: Bearer $TOKEN" \
  "$API/health"
```

Buscar servicios:

```sh
curl \
  -H "Authorization: Bearer $TOKEN" \
  "$API/services?search=worker&page=1&per_page=25"
```

Ejecutar una página de discovery:

```sh
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{}' \
  "$API/discovery"
```

La referencia completa de endpoints, parámetros y respuestas está disponible en [docs/API.md](docs/API.md).

---

## Fuente de verdad

Services Manager separa deliberadamente **metadata** de **estado operativo**.

SQLite contiene información administrada por el gestor:

* aliases
* descripciones
* tags
* visibilidad
* inventario y metadata de discovery

`systemd` mantiene la autoridad sobre el estado de las unidades.

Por ejemplo, si un servicio cambia de estado fuera de Services Manager:

```text
systemctl stop worker.service
```

la próxima consulta del servicio obtiene el estado directamente desde `systemd`, en lugar de confiar en un valor almacenado previamente en SQLite.

Del mismo modo, las operaciones de control siguen este orden:

```text
API request
    │
    ▼
systemd
    │
    ├── operación exitosa ──► actualizar metadata persistida
    │
    └── operación fallida ──► no asumir cambio de estado
```

---

## Instalación y permisos

Services Manager utiliza `systemctl` para consultar y controlar unidades.

Las operaciones que crean archivos bajo `/etc/systemd/system` o ejecutan `systemctl daemon-reload` requieren los permisos correspondientes.

Las plantillas `.service` creadas por la API incluyen nombre y descripción, pero **no incluyen `ExecStart`**. Debes completarlo antes de intentar iniciar la unidad.

La configuración detallada y las restricciones del contrato están documentadas en:

* [Control de systemd](docs/SYSTEM_CONTROL.md)
* [Contrato de API](docs/API.md)

---

## Desarrollo

Los scripts de desarrollo están disponibles para ambos sistemas:

```text
Linux    → build.sh / run.sh
Windows  → build.bat / run.bat
```

También puedes utilizar Docker para ejecutar las pruebas en un entorno reproducible:

```sh
docker compose build dev
docker compose run --rm dev cargo test --locked
docker compose run --rm dev cargo check --locked
```

El servicio `dev` utiliza proveedores de prueba y no requiere privilegios de `systemd`.

El perfil Docker de la API tampoco comparte el manager `systemd` del host: sirve para validar el servidor y la API, no el control real de unidades del sistema anfitrión.

---

## Documentación

| Documento                                                | Contenido                                     |
| -------------------------------------------------------- | --------------------------------------------- |
| [API](docs/API.md)                                       | Endpoints, contratos, parámetros y respuestas |
| [Discovery](docs/DISCOVERY.md)                           | Descubrimiento progresivo y reanudación       |
| [System Control](docs/SYSTEM_CONTROL.md)                 | Control de `systemd` y consistencia           |
| [Testing](docs/TESTING.md)                               | Estrategia y ejecución de pruebas             |
| [Docker](docs/DOCKER.md)                                 | Entorno Docker de desarrollo                  |
| [Architecture Decisions](docs/ARCHITECTURE_DECISIONS.md) | Decisiones y fundamentos de arquitectura      |

---

## Stack

**Rust · Axum · SQLite · systemd · Docker**

---

## Estado del proyecto

Services Manager está pensado como una capa de integración entre aplicaciones externas y el gestor de servicios de Linux, manteniendo una separación explícita entre la metadata administrada por la aplicación y el estado real del sistema.
