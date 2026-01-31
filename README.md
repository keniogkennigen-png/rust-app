# RustChat - Aplicación de Chat en Tiempo Real

Una aplicación de chat en tiempo real construida con Rust, utilizando el framework web Warp y WebSockets para comunicación bidireccional.

## Características

- **Registro y Autenticación**: Los usuarios pueden registrarse e iniciar sesión con un sistema seguro de hash de contraseñas (bcrypt).
- **Chat en Tiempo Real**: Mensajería instantánea usando WebSockets.
- **Gestión de Contactos**: Los usuarios pueden agregar contactos para chatear.
- **Indicadores de Estado**: Ver cuando los contactos están en línea o fuera de línea.
- **Indicadores de Escritura**: Ver cuando un contacto está escribiendo un mensaje.
- **Interfaz de Usuario Estilizada**: Diseño sci-fi moderno con Tailwind CSS.

## Requisitos

- Rust y Cargo (versión 1.56 o superior)
- Conexión a internet para cargar las dependencias

## Instalación

1. Clona el repositorio:
```bash
git clone https://github.com/keniogkennigen-png/rust-app.git
cd rust-app
```

2. Compila y ejecuta la aplicación:
```bash
cargo run
```

3. Abre tu navegador y visita:
```
http://localhost:3030
```

## Uso

1. **Registro**: Ingresa un nombre de usuario y contraseña, luego haz clic en "Establish Link".
2. **Agregar Contactos**: Una vez registrado, ingresa el nombre de otro usuario y haz clic en "Add Connection".
3. **Chat**: Selecciona un contacto de la lista para comenzar a chatear.

## Arquitectura

- **Backend**: Rust con Warp (framework web asíncrono)
- **Frontend**: HTML/CSS/JavaScript con Tailwind CSS
- **Comunicación**: WebSockets para mensajes en tiempo real, HTTP para autenticación y gestión de contactos
- **Almacenamiento**: En memoria (HashMaps) - los datos se pierden al reiniciar el servidor

## Rutas API

- `POST /register` - Registrar un nuevo usuario
- `POST /login` - Iniciar sesión
- `GET /contacts` - Obtener lista de contactos (requiere header `x-session-key`)
- `POST /contacts` - Agregar un contacto (requiere header `x-session-key`)
- `ws://host:3030/ws?token=SESSION_KEY` - Conexión WebSocket

## Licencia

MIT
