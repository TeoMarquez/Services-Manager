# Documentación del proyecto

Esta carpeta es el punto de entrada para retomar el trabajo de Services Manager. Describe el estado observado del código y el plan acordado; el código y las migraciones siguen siendo la fuente de verdad cuando haya diferencias.

## Documentos

- [Estado actual](CURRENT_STATE.md): qué está implementado y qué no.
- [Plan de desarrollo](DEVELOPMENT_PLAN.md): etapas y criterios de salida.
- [Decisiones de arquitectura](ARCHITECTURE_DECISIONS.md): reglas y decisiones que condicionan la implementación.
- [Discovery](DISCOVERY.md): páginas, checkpoint, reanudación y reconciliación de ausencias.
- [Control del sistema](SYSTEM_CONTROL.md): orden OS → DB, handshakes, verificación, compensación y recuperación.
- [Estrategia de pruebas](TESTING.md): pruebas unitarias, CLI, integración y entorno reproducible.
- [Docker](DOCKER.md): comandos para desarrollar y probar en contenedor.
- [API externa](API.md): rutas implementadas, contrato actual y pendientes de seguridad/cobertura.

## Prioridad inmediata

Prioridad: validar token `.env` y creación/recarga/discovery de plantillas en Linux real; cubrir fallos de verificación/rollback; fijar timeouts/permisos; y compartir casos de uso CLI/API. El estado y pendientes están en [Estado actual](CURRENT_STATE.md), [Plan](DEVELOPMENT_PLAN.md), [API](API.md) y [Decisiones](ARCHITECTURE_DECISIONS.md).
