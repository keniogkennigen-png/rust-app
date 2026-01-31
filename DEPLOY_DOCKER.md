# Despliegue con Docker ( cualquier servidor )

## Requisitos
- Docker instalado
- Docker Compose (opcional)

## Despliegue rápido

### Opción 1: Usando Docker Compose (recomendado)

```bash
# Clonar el repositorio
git clone https://github.com/keniogkennigen-png/rust-app.git
cd rust-app

# Construir y ejecutar
docker-compose up -d

# Ver logs
docker-compose logs -f

# Detener
docker-compose down
```

### Opción 2: Usando solo Docker

```bash
# Construir la imagen
docker build -t rust-chat .

# Ejecutar el contenedor
docker run -d -p 3030:3030 --name mi-rust-chat rust-chat

# Ver logs
docker logs -f mi-rust-chat

# Detener
docker stop mi-rust-chat
```

## Servidores donde puedes ejecutar Docker gratis o barato

### 1. Railway (gratis hasta cierto límite)
-railway.app
- Despliegue automático desde GitHub
- Dominio gratuito incluido

### 2. Render (gratis)
- render.com
- Web service gratuito con limitaciones
- Dominio personalizado opcional

### 3. Fly.io (gratis hasta límites)
- fly.io
- Très rápido y distribuido globalmente
- Dominio gratuito

### 4. Coolify (autohospedado gratis)
- coolify.io
- Instala en tu propio servidor
- Control total, sin límites

### 5. Portainer (gestión visual)
- portainer.io
- Interface gráfica para Docker
- Excelente para principiantes

## Verificar que funciona

```bash
# Verificar que el contenedor está ejecutándose
docker ps

# Probar endpoint local
curl http://localhost:3030
```

## Solución de problemas

### El puerto 3030 está ocupado
```bash
# Cambiar puerto en docker-compose.yml
ports:
  - "8080:3030"  # Usar puerto 8080
```

### La aplicación no responde
```bash
# Ver logs del contenedor
docker-compose logs

# Reiniciar contenedor
docker-compose restart
```

### Actualizar la aplicación
```bash
# Hacer pull de cambios
git pull origin main

# Reconstruir y reiniciar
docker-compose up -d --build
```
