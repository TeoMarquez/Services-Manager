# Plan de desarrollo

## Principios de ejecución

- La DB es una proyección del estado del sistema operativo, no la autoridad para controlar servicios.
- Discovery avanza por etapas pequeñas, observables e idempotentes; los hallazgos parciales deben poder consultarse y retomarse.
- No afirmar que una operación fue exitosa sin verificarla en el proveedor.
- Probar cada capa con proveedor mock antes de habilitar cambios reales en Linux.
- Mantener comandos equivalentes de desarrollo/prueba en Windows y Linux mediante Docker.

## Etapa 0 — Contratos y límites (parcial)

- Definir estados descubiertos, presentes/ausentes, estados operacionales y metadatos.
- Definir qué significa reset y qué operaciones mutan el sistema.
- Definir mecanismo de control Linux y disponibilidad/permisos esperados.
- Definir transporte de API, ciclo de vida, autenticación y compatibilidad.
- Registrar las decisiones y sus consecuencias antes de cerrar cada contrato.

**Salida:** regla OS→verificación→DB→compensación y reset limitado a metadata están implementados; faltan cerrar permisos, timeout, estados systemd admitidos y recuperación tras caída.

## Etapa 1 — Discovery incremental y lectura de DB (base mock implementada)

- Separar enumeración, lectura/detalle y persistencia/reconciliación.
- Definir cursor/lotes o etapas para que el discovery pueda progresar y reanudarse.
- Registrar altas y observaciones actualizadas; marcar ausencias solo al completar un ciclo válido, nunca por un lote parcial.
- Hacer idempotentes las repeticiones y proteger concurrencia/ciclos superpuestos.
- Exponer consultas/búsqueda por servicios, presencia, visibilidad, tipo y tags usando filtros parametrizados.
- Extender el mock para simular lotes, errores y desapariciones.

**Salida:** discovery mock y systemd por snapshot, DB coherente tras ciclo completo/interrumpido/repetido, búsqueda/filtros/tags consumibles por servicios y API. La CLI no tiene todos los filtros/tags como comandos automatizables.

## Etapa 2 — Pruebas (núcleo inicial implementado)

- Añadir tests unitarios para transformación, filtros, paginación, tags y reglas de reconciliación.
- Añadir tests SQLite de integración para migraciones y repositorios en DB temporal.
- Añadir pruebas de CLI automatizables para entradas, salidas, errores y flujos principales.
- Añadir pruebas de control con proveedor falso que inyecte fallos antes/después de cada handshake.
- Ejecutar la misma suite dentro del contenedor de desarrollo.

**Salida:** suite reproducible en Windows y Docker Linux (18 unitarios + 1 CLI). Matriz OS/DB está cubierta parcialmente; fallan/verificación/rollback indeterminado aún faltan.

## Etapa 3 — Docker y desarrollo Linux (desarrollo/tests mock implementados)

- Incorporar Dockerfile multi-stage y Compose con volúmenes de datos/código apropiados.
- Separar el contenedor de desarrollo/pruebas del entorno de integración systemd.
- No asumir que un contenedor común ejecuta systemd; documentar requisitos y alternativa de integración.
- Evitar montar sockets privilegiados en el perfil normal de tests.

**Salida:** compilación y suite reproducibles con `docker compose`; no se montan sockets/privilegios del host. El contenedor no valida control systemd real.

## Etapa 4 — Control real de servicios (base implementada; validación Linux pendiente)

- Implementar proveedor Linux detrás del trait. `SystemdProvider` usa `systemctl` para inventario, estado, start/stop y enable/disable.
- Usar el protocolo OS → verificación → DB → compensación definido en `SYSTEM_CONTROL.md`. Coordinador y persistencia de estado operacional implementados.
- Tratar estado previo/desconocido, timeout, permisos, servicio desaparecido y fallo de rollback.
- Mantener la observación de discovery separada de comandos de control.

**Salida:** base de operaciones controladas verificables implementada. Faltan tests de estado indeterminado/rollback fallido y validación contra systemd real. Los estados `masked/static` no se traducen a enabled/disabled.

## Etapa 5 — API externa (base REST implementada; contrato/cobertura pendientes)

- Definir contrato versionado y modelo de errores.
- Enrutar comandos a servicios de aplicación, nunca directamente a SQL.
- Añadir discovery, búsqueda/listado, visibilidad, tags, reset y control operacional. Las rutas REST start/stop/startup-mode usan el coordinador.
- Añadir límites de concurrencia, timeout, autorización y protección ante solicitudes repetidas.
- Cubrir contrato y flujos de éxito/compensación con pruebas.

**Salida:** API REST/JSON con token bearer y rutas de discovery, búsqueda, visibilidad, tags, reset, start/stop y startup-mode. El contrato aún requiere endurecer autorización/concurrencia, timeouts y cobertura; la prueba de start usa proveedor falso.

## Secuencia inmediata

1. Validar `SystemdProvider` y comandos `systemctl` en Linux con systemd real; precisar permisos y timeouts.
2. Completar tests del coordinador para verificación, rollback fallido, parcialidad, indeterminación e idempotencia; cubrir todas las rutas de control API.
3. Extraer casos de uso compartidos para CLI/API y sumar subcomandos no interactivos donde correspondan.
4. Cerrar contrato de seguridad y concurrencia de la API, discovery/reset repetible y recuperación ante caída OS/DB.

## Registro de avance

- 2026-09-29: análisis inicial del repositorio; confirmada la rama `agent/continue`; documentación de contexto creada como primera entrega.
- 2026-09-29: proveedor de discovery propaga errores; la migración 03 guarda el cursor de ciclo y las unidades observadas. `sync_step` procesa lotes y reanuda tras fallo; solo completa ausencias al cerrar el ciclo.
- 2026-09-29: búsqueda literal parametrizada, filtros combinables y validación de paginación disponibles; búsqueda integrada en CLI. 9 tests unitarios y 1 CLI pasan localmente y en Docker Linux.
- 2026-09-29: el test CLI Linux detectó que el volumen target inicial contenía el binario stub del Dockerfile; se corrigió compilando el código real y usando un target volume nuevo. Docker build y tests ya pasan.
- 2026-09-29: filtros de servicio se pueden combinar tipadamente; búsqueda escapa comodines y los valores van parametrizados. Migración idempotente y rollback atómico por trigger cubiertos.
- 2026-09-29: añadidos `SystemdProvider`, coordinador OS → verificación → DB → compensación, migración 04 para estado operacional y rutas API start/stop/startup-mode. Tests cubren éxito, fallo OS, compensación tras fallo DB y ruta API start; `cargo test --offline` y Docker Linux pasan (18 unitarios + 1 CLI).
- 2026-09-29: el proveedor systemd admite los códigos de salida de `is-enabled` para estados normales como `disabled` y conserva una instantánea del inventario en el cursor de discovery para que el ciclo no dependa de offsets sobre una lista cambiante.
- Próximo: cubrir fallos de verificación/rollback e implementar timeouts; validar proveedor en Linux con systemd real; compartir casos de uso CLI/API y cerrar seguridad/concurrencia.
