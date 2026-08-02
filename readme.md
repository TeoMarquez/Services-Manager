# Services Manager

## Descripción

Services Manager es un gestor de servicios del sistema operativo diseñado para centralizar la administración, persistencia y consulta de servicios.

El proyecto mantiene una separación clara entre:

* La persistencia de información.
* La lógica de gestión de servicios.
* La comunicación con el sistema operativo.
* Las interfaces de interacción.

El objetivo principal es proporcionar una capa de gestión independiente que pueda ser utilizada por distintas interfaces externas sin acoplar la lógica interna a una implementación concreta.

---

# Arquitectura

El proyecto está dividido en módulos con responsabilidades independientes:

```
services-manager

├── db
│   ├── models
│   └── repositories
│
├── services
│   ├── sync
│   ├── visibility
│   ├── deletion
│   ├── tagging
│   └── querying
│
├── system
│   └── provider
│
└── cli
    └── testing interface
```

---

# Módulo DB

Responsable de la persistencia y consulta de información.

Incluye:

* Modelos de datos.
* Repositorios.
* Consultas SQL.
* Relaciones entre entidades.

Actualmente utiliza SQLite mediante `rusqlite`.

## Responsabilidades

La capa de base de datos administra:

* Servicios registrados.
* Estado de presencia.
* Visibilidad.
* Metadata asociada.
* Tags y relaciones entre servicios.

## No es responsabilidad de la DB:

* Ejecutar servicios.
* Modificar el sistema operativo.
* Conocer interfaces externas.
* Implementar lógica de presentación.

La base de datos únicamente representa el estado persistido del gestor.

---

# Módulo System Provider

Este módulo representa la comunicación con el sistema operativo.

Su responsabilidad es abstraer el proveedor real de servicios.

Actualmente existe:

```
MockSystem
```

utilizado para pruebas y desarrollo.

La implementación futura reemplazará esta abstracción por un proveedor real basado en el sistema de servicios correspondiente.

La capa superior no debe depender de una implementación concreta, sino de las capacidades ofrecidas por este módulo.

---

# Service Manager Core

Es el núcleo lógico del proyecto.

Su responsabilidad es coordinar:

```
Sistema operativo
        │
        ▼
Service Manager
        │
        ▼
Base de datos
```

El gestor mantiene consistencia entre el estado real del sistema y el estado persistido.

Ejemplos:

* Sincronizar servicios detectados.
* Registrar nuevos servicios.
* Marcar servicios ausentes.
* Actualizar visibilidad.
* Gestionar etiquetas.
* Ejecutar consultas filtradas.

---

# Funcionalidades implementadas

## Sincronización

Permite comparar los servicios existentes en el sistema con los registrados en la base de datos.

Actualmente permite:

* Detectar nuevos servicios.
* Registrar servicios encontrados.
* Mantener información de presencia.

---

## Gestión de visibilidad

Permite controlar qué servicios aparecen dentro de las consultas del gestor.

Operaciones disponibles:

* Mostrar servicio.
* Ocultar servicio.

La visibilidad es metadata propia del gestor y no modifica el servicio real del sistema operativo.

---

## Eliminación

Permite eliminar registros de servicios gestionados.

La operación actualmente trabaja sobre la capa persistente.

La futura integración con el proveedor del sistema permitirá sincronizar esta acción con la eliminación real del servicio.

---

## Sistema de etiquetas

Los servicios pueden clasificarse mediante tags.

Implementado:

* Crear etiquetas.
* Listar etiquetas.
* Asociar etiquetas a servicios.
* Eliminar asociaciones.
* Consultar etiquetas de un servicio.
* Buscar servicios mediante etiquetas.

Modelo:

```
services

    │
    │
service_tags
    │
    │
tags
```

---

## Consultas y filtros

Se implementó una capa de construcción de consultas para evitar duplicación de lógica.

Permite filtrar servicios por:

* Todos.
* Presentes.
* Ausentes.
* Servicios del sistema.
* Servicios de usuario.
* Visibles.
* Ocultos.
* Etiquetas.

La generación de filtros está separada de la ejecución SQL para mantener las responsabilidades divididas.

---

# CLI

La CLI incluida actualmente es una herramienta de prueba y administración manual.

No representa la interfaz final del sistema.

Permite verificar:

* Sincronización.
* Listado.
* Visibilidad.
* Eliminación.
* Gestión de tags.

Su función principal es validar el comportamiento del núcleo durante el desarrollo.

---

# Decisiones arquitectónicas

## Separación de responsabilidades

Cada módulo tiene una responsabilidad concreta:

```
DB
↓
Persistencia

System Provider
↓
Comunicación con el sistema operativo

Service Manager
↓
Reglas y coordinación

Interfaces externas
↓
Interacción con usuarios o aplicaciones
```

---

## El núcleo no conoce consumidores externos

Services Manager no depende de ninguna interfaz concreta.

No conoce:

* Aplicaciones cliente.
* Interfaces gráficas.
* APIs.
* Automatizaciones externas.

Cualquier consumidor debe implementar su propia capa de comunicación y transformación de datos.

---

## Evitar acoplamiento

Las capas superiores no deben depender de detalles internos.

Ejemplo:

Cambiar SQLite por otra base de datos no debería modificar la lógica de gestión.

Cambiar el proveedor del sistema operativo no debería modificar consultas, tags o reglas del gestor.

---

# Estado actual

## Etapa completada

* Modelo inicial de datos.
* Repositorios.
* Sincronización.
* Gestión de presencia.
* Gestión de visibilidad.
* Eliminación.
* Sistema de tags.
* Filtros de consulta.
* Paginación.
* CLI interactiva de pruebas.

---

# Próximas etapas

## Provider real del sistema

Implementación de operaciones reales:

* Descubrimiento de servicios.
* Creación.
* Modificación.
* Eliminación.
* Consulta de estado.

---

## Capa de comunicación externa

Definición de una interfaz estable para permitir que otros sistemas consuman el gestor sin acceder directamente a sus componentes internos.

---

# Principio general del proyecto

Services Manager debe actuar como una unidad independiente de gestión de servicios.

La arquitectura busca que cada componente pueda evolucionar sin arrastrar cambios innecesarios sobre el resto del sistema.
