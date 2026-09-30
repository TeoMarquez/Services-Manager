# Decisiones de arquitectura

## ADR-001 — El sistema operativo es la autoridad del estado operacional

**Estado:** Acordado.

La DB refleja observaciones/estado confirmado del sistema operativo. No se escribe primero en DB para intentar después controlar el servicio. Todo cambio operacional sigue el orden OS → verificación → DB.

## ADR-002 — Compensación cuando falla la persistencia

**Estado:** Acordado e implementado en primera iteración; cobertura de fallos extremos y validación de sistema real pendientes.

Antes de mutar el sistema se captura en memoria el estado previo y la acción inversa posible. Tras ejecutar y verificar la acción en el sistema, se persiste la observación. Si la persistencia falla, se intenta restaurar el estado previo en el sistema y se verifica la compensación. Un fallo de compensación se informa como divergencia que requiere reconciliación; no se oculta ni se reporta éxito.

La transacción SQLite no puede ser atómica con una operación externa del sistema operativo. Existe una ventana de caída del proceso entre el cambio del sistema y la confirmación en DB. Se requiere reconciliación en el próximo discovery; un journal durable podría reducir esa ventana, pero su ubicación/semántica deben respetar la regla OS-first y definirse antes de adoptarlo.

## ADR-003 — Discovery en lotes y ausencia solo al completar ciclo

**Estado:** Implementado para proveedores mock y systemd.

Los resultados de lotes se guardan con un checkpoint durable. Un servicio solo se marca ausente cuando una enumeración completa terminó correctamente. Fallos parciales no equivalen a ausencia. El cursor debe distinguir una ejecución completa de una interrumpida y el proveedor debe mantener coherente el inventario lógico del ciclo.

## ADR-004 — Proveedor intercambiable

**Estado:** Existente en código.

El trait `SystemProvider` separa consumidor y proveedor. `MockSystem` soporta discovery de fixture y `SystemdProvider` consulta/controla unidades Linux; los tests usan implementaciones controlables. La validación de integración systemd real está pendiente.

## Decisiones por resolver

- Linux: systemd es la implementación inicial; falta fijar versión mínima y validar el contrato de `systemctl`.
- API: REST/JSON se implementa como supuesto inicial; faltan cerrar modelo de error, límites, concurrencia y seguridad operacional.
- Control: el modelo actual representa enabled/disabled; decidir si se amplía para masked/runtime/static y fijar permisos/timeouts.
- Reset: la ruta actual limpia servicios, tags y checkpoint, sin tocar OS; confirmar si API requiere protección adicional contra llamadas accidentales.
- Creación: el API genera plantillas unit file sin `ExecStart` (según requisito); el archivo se crea sin sobrescribir, se hace daemon-reload y discovery. La ruta configurada debe pertenecer al unit search path de systemd para que el proveedor la encuentre.
- Modelo de discovery: cursor/snapshot implementados; evaluar tamaño del cursor con inventario real y mejorar concurrencia/ciclo abortado.
- Persistencia del protocolo de operaciones para recuperación tras caída del proceso.
- Permisos mínimos/roles, límites de exposición, rate limiting y concurrencia de API.

## Valores iniciales propuestos para avanzar

**Estado:** Defaults iniciales implementados; mantenerlos revisables hasta cerrar contrato externo.

- API REST/JSON versionada; bind local `127.0.0.1` por defecto.
- Proveedor Linux basado en systemd, coherente con la fixture `mock/systemd.json`.
- `reset` limitado a datos administrados por Services Manager; nunca resetea unidades del SO.
- Un solo token bearer compartido, guardado en `.env`; se genera si falta y solo rota al solicitar `--overwrite-token`.

No exponer la API en red hasta cerrar autorización por operación, permisos/timeouts y validar el handshake de `SYSTEM_CONTROL.md` con systemd real.
