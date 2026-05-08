# Invidious with Nginx Proxy Manager - Embed Whitelist Setup

## Problem

When Invidious is protected by Basic Auth, embedding videos in iframes fails because:
1. Browsers block subresource requests with credentials in the URL (`https://user:pass@host/`)
2. The Basic Auth popup appears when loading the iframe

## Solution

Configure Nginx Proxy Manager to **whitelist `/embed/` paths** - allow embed requests without authentication while keeping everything else protected.

## Nginx Proxy Manager Configuration

1. **Go to NPM web UI** (typically `http://your-server:81`)

2. **Edit your Invidious proxy host**

3. **Advanced tab** - Add this custom Nginx configuration:

   ```nginx
   # Allow embed paths without authentication
   location /embed/ {
       proxy_pass http://invidious:3000;
       proxy_set_header Host $host;
       proxy_set_header X-Real-IP $remote_addr;
       proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
       proxy_set_header X-Forwarded-Proto $scheme;
   }
   ```

4. **Details tab** - Keep your existing settings:
   - Domain Names: `inv.wandabanet.de`
   - Scheme: `http`
   - Forward Hostname / IP: `invidious` (or your container name/IP)
   - Forward Port: `3000`

5. **Access tab** - Keep Basic Auth enabled here (protects all non-embed paths)

6. **Save**

## How It Works

Nginx processes locations by **specificity** (most specific first):
- `/embed/` location → Direct proxy, NO auth required
- Everything else → Falls through to NPM's Access auth

## Verification

Test with curl:

```bash
# Should return 200 (no auth required)
curl -I https://inv.wandabanet.de/embed/VIDEO_ID

# Should return 401 (auth required)
curl -I https://inv.wandabanet.de/api/v1/videos/VIDEO_ID

# Should return 200 (with auth)
curl -I -u username:password https://inv.wandabanet.de/api/v1/videos/VIDEO_ID
```

## twitch-relay Configuration

After setting up NPM whitelist:

1. **Keep** `INVIDIOUS_BASIC_AUTH_USER` and `INVIDIOUS_BASIC_AUTH_PASSWORD` in `.env`
   - Required for backend API calls (subscriptions, playlists, etc.)

2. **Frontend** automatically constructs clean embed URLs without credentials
   - Example: `https://inv.wandabanet.de/embed/VIDEO_ID?autoplay=1&quality=dash`

3. **Rebuild frontend** after code changes:
   ```bash
   cd web && pnpm run build
   ```

## Security Considerations

- Embed endpoints are publicly accessible (required for iframe loading)
- All other Invidious endpoints remain protected by Basic Auth
- Backend API calls still use credentials from environment variables
- Credentials never exposed in frontend code or URLs
