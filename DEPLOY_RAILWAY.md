# Despliegue de RustChat en Railway (Gratis)

## Pasos para desplegar en Railway:

### 1. Preparar tu cuenta de GitHub
Tu código ya está en: https://github.com/keniogkennigen-png/rust-app

### 2. Desplegar en Railway

1. **Ve a railway.app** y crea una cuenta gratuita (puedes usar GitHub para login)

2. **Haz clic en "New Project"** y selecciona "Deploy from GitHub repo"

3. **Selecciona tu repositorio** `keniogkennigen-png/rust-app`

4. **Railway detectará automáticamente** el Dockerfile y configurará todo

5. **Espera a que termine el despliegue** (puede tomar 2-5 minutos la primera vez)

6. **Railway te asignará un dominio gratuito** como:
   ```
   https://rust-chat-production.up.railway.app
   ```

7. **¡Listo!** Tu app estará online las 24 horas

### 3. Configuración de variables de entorno (opcional)
Si necesitas configurar variables de entorno, ve a la pestaña "Variables" en Railway y agrega:
- `PORT` = 3030
- `HOST` = 0.0.0.0

### 4. Verificar que funciona
Una vez desplegado, visita tu URL y你应该 poder:
- Registrarte
- Iniciar sesión
- Chatar con otros usuarios

## Despliegue alternativo en Render.com

1. Ve a render.com y crea cuenta gratuita
2. Haz clic en "New +" → "Web Service"
3. Conecta tu repositorio de GitHub
4. Configura:
   - Build Command: `cargo build --release`
   - Start Command: `./target/release/rust_chat`
5. Haz clic en "Create Web Service"

Render también te dará un dominio gratuito.
