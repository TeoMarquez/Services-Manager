# Discovery por ciclos y páginas

## Estado del ciclo

La migración `03_discovery_checkpoint.sql` crea una fila singleton con estado activo, cursor opaco y contadores del ciclo. `discovery_seen` contiene las unidades observadas en el ciclo actual. Al iniciar una ejecución se abre un ciclo nuevo si no hay uno activo; si el último ciclo quedó activo tras un error o reinicio, se reanuda desde su cursor.

## Contrato de página del proveedor

`SystemProvider::list_services_page(cursor, limit)` devuelve una `ServicePage` con servicios, cursor siguiente opcional y bandera `complete`.

- La página no puede exceder el límite.
- Una página incompleta debe contener servicios y avanzar a un cursor distinto.
- La página final declara `complete = true` y no incluye siguiente cursor.
- Los cursores son opacos para la capa de aplicación.
- El proveedor debe asegurar que el cursor se pueda retomar después de reiniciar el proceso y que las páginas de un mismo ciclo refieran al mismo inventario lógico. El token debe identificar el snapshot/generación cuando la fuente pueda cambiar entre páginas.

El mock pagina el JSON por offset porque su inventario es una fixture local estática. `SystemdProvider` toma `list-unit-files` al inicio del ciclo y serializa el listado ordenado más el offset en el cursor para mantener el snapshot al reanudar. Esto mantiene consistencia pero el token durable es proporcional al número/tamaño de las unidades.

## Persistencia y ausencia

Cada página realiza en una transacción: insertar/actualizar servicios, añadirlos a `discovery_seen`, mover cursor y acumular contadores. Así, un reinicio no puede avanzar el cursor sin guardar sus observaciones ni repetir una página puede duplicar servicios.

Solo la página final pone a ausentes los servicios no observados y cierra el ciclo, en la misma transacción. Una falla del proveedor conserva el ciclo activo en el cursor anterior; la próxima llamada puede reintentar.

La tabla singleton y una comparación del cursor al guardar una página evitan que dos llamadas concurrentes sobrescriban silenciosamente el mismo avance. Una llamada que perdió la carrera falla y debe volver a leer el checkpoint.

## Interfaces

- `sync_step(provider, repo, limit)` procesa una sola página y devuelve el checkpoint/progreso.
- `sync(provider, repo)` itera páginas hasta completar y devuelve el reporte acumulado.
- La CLI usa `sync`; `POST /api/v1/discovery` usa `sync_step` y responde progreso de una página por petición.

## Límites actuales

- El mock vuelve a leer y parsear todo el JSON por página; sirve para validar cursor y reanudación, no rendimiento.
- No hay pausa/cancelación explícita, retención de historial de ciclos ni gestión para abortar un cursor permanentemente inválido.
- El error del proveedor deja el ciclo en curso para reintento. Un mecanismo futuro debe permitir resetear/abortar el ciclo tras cambio incompatible de proveedor/schema sin marcar servicios como ausentes.
- `SystemdProvider` captura el conjunto de unit files en el cursor durable y pagina esa misma instantánea durante el ciclo. Cambios posteriores en systemd se reflejan en el próximo ciclo, evitando omisiones por offsets sobre un inventario cambiante. Falta validar en host Linux y evaluar una alternativa al cursor grande si el inventario lo requiere.
