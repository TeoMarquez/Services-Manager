# Control de servicios y consistencia OS/DB

## Regla obligatoria

El sistema operativo es la autoridad operacional y la base representa el estado confirmado. Para iniciar, detener o cambiar el modo de inicio, el orden es:

1. Leer y conservar el estado previo desde el proveedor.
2. Validar que la transición sea posible y preparar la compensación.
3. Ejecutar la operación en el sistema operativo.
4. Consultar nuevamente el sistema y verificar el estado solicitado.
5. Persistir en una transacción SQLite la observación confirmada.
6. Confirmar el resultado al consumidor.

Nunca informar éxito antes del paso 5. Nunca cambiar primero la DB para luego intentar cambiar el sistema.

## Fallos y handshake

- **Falla antes del cambio OS:** DB intacta; devolver el error del proveedor.
- **OS devuelve error:** volver a consultar para determinar si hubo efecto parcial. Si se detectó un cambio, compensarlo cuando sea seguro y verificarlo.
- **OS cambia pero verificación falla:** no escribir estado deseado en DB; intentar recuperar el estado anterior y reportar resultado/indeterminación.
- **Verificación OS exitosa, escritura DB falla:** emitir la operación inversa basada en el estado capturado, verificar rollback y conservar el error de DB.
- **Rollback falla:** devolver un error compuesto con la operación inicial, fallo de persistencia y fallo de compensación; señalar que el estado requiere discovery/reconciliación. No ocultar la divergencia.
- **Timeout o estado no concluyente:** no asumir éxito ni fallo; consultar de nuevo y clasificar como confirmado, compensado o indeterminado.

## Implementación actual y brechas

- `SystemdProvider` implementa consulta, start/stop y enable/disable mediante `systemctl`; `control` captura estado previo, vuelve a leer después de mutar y persiste solo la observación que coincide con lo solicitado.
- El repositorio guarda `active`, modo de inicio y `observed_at` en `service_operational_state`, asociado a la fila de servicio.
- Si la acción OS falla, el coordinador intenta restaurar estado previo; si falla la lectura posterior, si la observación no coincide o falla SQLite, intenta compensar y verifica el resultado. El error expone el texto de compensación.
- La cobertura automatizada actual verifica éxito, error OS, error SQLite y rollback exitoso. Faltan inyección de cambio parcial OS, lectura de verificación fallida, rollback fallido, idempotencia y timeout.
- Las invocaciones actuales a `systemctl` no tienen límite de tiempo. El API no debe exponerse a una red hasta acordar permisos/roles y validar el comportamiento en Linux real.
- Estados systemd no expresables en el modelo actual (`failed`, `masked`, `static`, `enabled-runtime` y estados transitorios) se rechazan; aún falta validar parsing/códigos de salida en una máquina con systemd.

La interfaz de aplicación debe conservar suficiente contexto para explicar cada resultado: estado previo, solicitado, observado, persistido y resultado de compensación.

## Ventana de caída y recuperación

SQLite y systemd no comparten una transacción distribuida. Si el proceso cae después de aplicar el cambio OS y antes de persistir DB, la DB queda temporalmente atrasada. El siguiente discovery completo debe volver a observar el SO y reconciliar la DB.

Antes de implementar control real hay que decidir si esa ventana exige un journal durable. Si se adopta, el journal debe registrar una intención/operación y su fase sin convertir a la DB de servicios en autoridad operacional ni presentar el comando como completado antes de verificar el SO. Definir almacenamiento, recuperación, retención y tratamiento de órdenes repetidas.

## Separación de responsabilidades

- El proveedor consulta y controla el SO; no escribe DB.
- El servicio de aplicación coordina handshake, verificación, persistencia y compensación.
- El repositorio persiste observaciones ya confirmadas; no ejecuta comandos de sistema.
- La API/CLI convierte solicitudes a comandos de aplicación y muestra errores con estado de recuperación.
- Discovery observa/reconcilia; no debe inferir que una operación fue exitosa solo por la DB.

## Pruebas requeridas

Usar un proveedor falso con estado observable e inyección de fallos para verificar el orden de llamadas y estos casos: éxito completo, fallo OS, verificación fallida, fallo DB y rollback exitoso, rollback fallido, timeout/estado indeterminado y repetición idempotente.
