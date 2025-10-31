#!/bin/bash

# Tic Tac Flic Flac Floe - Create Redistribution Package
# This script creates a complete redistribution package with the built website
# and deployment scripts that can be used to deploy to any location

set -e

echo "==============================================="
echo "Creating Redistribution Package"
echo "==============================================="
echo ""

# Clean and build
echo "[1/4] Building website with Trunk..."
trunk build --release

if [ ! -d "dist" ]; then
    echo "Error: Build failed, dist directory not found"
    exit 1
fi

# Create package directory
PACKAGE_DIR="ttfff-redist-package"
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR"

echo "[2/4] Copying built website files..."
cp -r dist/* "$PACKAGE_DIR/"

echo "[3/4] Adding deployment scripts..."

# Create the deployment script for the package
cat > "$PACKAGE_DIR/deploy.sh" << 'DEPLOY_SH_EOF'
#!/bin/bash

# Deployment script for Tic Tac Flic Flac Floe
# Usage: ./deploy.sh [base-url]
# Example: ./deploy.sh https://example.com/games/ttfff

set -e

if [ -z "$1" ]; then
    echo "Usage: ./deploy.sh [base-url]"
    echo "Example: ./deploy.sh https://example.com/games/ttfff"
    echo ""
    echo "Use './' or '' for root deployment"
    exit 1
fi

BASE_URL="$1"

# Remove trailing slash
BASE_URL="${BASE_URL%/}"

echo "Configuring deployment for: $BASE_URL"
echo ""

# Update manifest.json
if [ -f "manifest.json" ]; then
    if [ "$BASE_URL" = "" ] || [ "$BASE_URL" = "." ]; then
        sed -i.bak 's|"start_url": ".*"|"start_url": "/"|g' manifest.json
    else
        sed -i.bak "s|\"start_url\": \".*\"|\"start_url\": \"$BASE_URL\"|g" manifest.json
    fi
    rm -f manifest.json.bak
    echo "✓ Updated manifest.json"
fi

# Update index.html base tag and service worker registration
if grep -q '<base href=' index.html; then
    if [ "$BASE_URL" = "" ] || [ "$BASE_URL" = "." ]; then
        sed -i.bak 's|<base href="[^"]*"|<base href="/"|g' index.html
    else
        sed -i.bak "s|<base href=\"[^\"]*\"|<base href=\"$BASE_URL/\"|g" index.html
    fi
else
    if [ "$BASE_URL" = "" ] || [ "$BASE_URL" = "." ]; then
        sed -i.bak 's|<head>|<head>\n    <base href="/">|' index.html
    else
        sed -i.bak "s|<head>|<head>\n    <base href=\"$BASE_URL/\">|" index.html
    fi
fi

# Update service worker registration path
if [ "$BASE_URL" = "" ] || [ "$BASE_URL" = "." ]; then
    sed -i.bak "s|navigator.serviceWorker.register('[^']*')|navigator.serviceWorker.register('/sw.js')|g" index.html
else
    sed -i.bak "s|navigator.serviceWorker.register('[^']*')|navigator.serviceWorker.register('$BASE_URL/sw.js')|g" index.html
fi
rm -f index.html.bak
echo "✓ Updated index.html"

# Update service worker cache URLs
if [ -f "sw.js" ]; then
    if [ "$BASE_URL" = "" ] || [ "$BASE_URL" = "." ]; then
        sed -i.bak "s|urlsToCache = \[|urlsToCache = [|g" sw.js
    else
        # Update all cached URLs to include base path
        sed -i.bak "s|'/'|'$BASE_URL/'|g" sw.js
        sed -i.bak "s|'/index.html'|'$BASE_URL/index.html'|g" sw.js
        sed -i.bak "s|'/manifest.json'|'$BASE_URL/manifest.json'|g" sw.js
    fi
    rm -f sw.js.bak
    echo "✓ Updated sw.js"
fi

echo ""
echo "Configuration complete!"
echo "Files are ready to deploy to: $BASE_URL"
echo ""
echo "Next steps:"
echo "1. Upload all files in this directory to your web server"
echo "2. Configure your web server (see WEB_SERVER_CONFIG.txt)"
DEPLOY_SH_EOF

chmod +x "$PACKAGE_DIR/deploy.sh"

# Create Windows deployment script
cat > "$PACKAGE_DIR/deploy.bat" << 'DEPLOY_BAT_EOF'
@echo off
setlocal enabledelayedexpansion

REM Deployment script for Tic Tac Flic Flac Floe
REM Usage: deploy.bat [base-url]
REM Example: deploy.bat https://example.com/games/ttfff

if "%~1"=="" (
    echo Usage: deploy.bat [base-url]
    echo Example: deploy.bat https://example.com/games/ttfff
    echo.
    echo Use "./" or "" for root deployment
    exit /b 1
)

set "BASE_URL=%~1"

REM Remove trailing slash
if "%BASE_URL:~-1%"=="/" set "BASE_URL=%BASE_URL:~0,-1%"

echo Configuring deployment for: %BASE_URL%
echo.

REM Update manifest.json
if exist manifest.json (
    if "%BASE_URL%"=="" (
        powershell -Command "(Get-Content 'manifest.json') -replace '\"start_url\": \".*\"', '\"start_url\": \"/\"' | Set-Content 'manifest.json'"
    ) else (
        powershell -Command "(Get-Content 'manifest.json') -replace '\"start_url\": \".*\"', '\"start_url\": \"%BASE_URL%\"' | Set-Content 'manifest.json'"
    )
    echo [OK] Updated manifest.json
)

REM Update index.html
if "%BASE_URL%"=="" (
    powershell -Command "$content = Get-Content 'index.html' -Raw; if ($content -match '<base href=') { $content = $content -replace '<base href=\"[^\"]*\"', '<base href=\"/\">' } else { $content = $content -replace '<head>', '<head>\n    <base href=\"/\">' }; $content = $content -replace 'navigator.serviceWorker.register\(''[^'']*''\)', 'navigator.serviceWorker.register(''/sw.js'')'; Set-Content 'index.html' -Value $content"
) else (
    powershell -Command "$content = Get-Content 'index.html' -Raw; if ($content -match '<base href=') { $content = $content -replace '<base href=\"[^\"]*\"', '<base href=\"%BASE_URL%/\">' } else { $content = $content -replace '<head>', '<head>\n    <base href=\"%BASE_URL%/\">' }; $content = $content -replace 'navigator.serviceWorker.register\(''[^'']*''\)', 'navigator.serviceWorker.register(''%BASE_URL%/sw.js'')'; Set-Content 'index.html' -Value $content"
)
echo [OK] Updated index.html

REM Update service worker
if exist sw.js (
    if "%BASE_URL%"=="" (
        REM Root deployment - keep absolute paths
        echo [OK] Service worker configured for root deployment
    ) else (
        REM Update service worker cache URLs
        powershell -Command "(Get-Content 'sw.js') -replace '''/'',', ''%BASE_URL%/',' -replace '''/index.html''', '''%BASE_URL%/index.html''' -replace '''/manifest.json''', '''%BASE_URL%/manifest.json''' | Set-Content 'sw.js'"
        echo [OK] Updated sw.js
    )
)

echo.
echo Configuration complete!
echo Files are ready to deploy to: %BASE_URL%
echo.
echo Next steps:
echo 1. Upload all files in this directory to your web server
echo 2. Configure your web server (see WEB_SERVER_CONFIG.txt)

endlocal
DEPLOY_BAT_EOF

# Create web server configuration guide
cat > "$PACKAGE_DIR/WEB_SERVER_CONFIG.txt" << 'CONFIG_EOF'
Web Server Configuration for Tic Tac Flic Flac Floe
====================================================

The app requires URL rewriting to work correctly as a Single Page Application (SPA).

APACHE
------
Create or edit .htaccess in the deployment directory:

    RewriteEngine On
    RewriteBase /
    RewriteCond %{REQUEST_FILENAME} !-f
    RewriteCond %{REQUEST_FILENAME} !-d
    RewriteRule ^(.*)$ index.html [L]

If deploying to a subdirectory (e.g., /games/ttfff/), change RewriteBase:

    RewriteBase /games/ttfff/


NGINX
-----
Add to your location block:

    location /games/ttfff/ {
        alias /path/to/deployment/;
        try_files $uri $uri/ /games/ttfff/index.html;
    }

For root deployment:

    location / {
        root /path/to/deployment/;
        try_files $uri $uri/ /index.html;
    }


STATIC FILE SERVERS (Python, Node.js, etc.)
--------------------------------------------
Most simple static servers don't support SPA routing by default.

Python (using python3 -m http.server):
    Not recommended for production, but works for local testing.
    Use --directory option to serve from the deployment folder.

Node.js (using serve package):
    npm install -g serve
    serve -s . -l 8080
    
    The -s flag enables SPA mode (rewrites to index.html)


MIME TYPES
----------
Ensure your server serves .wasm files with correct MIME type:
    application/wasm

Most modern servers handle this automatically.


HTTPS REQUIREMENT
-----------------
Progressive Web App (PWA) features require HTTPS.
For local testing, use localhost which is treated as secure.
CONFIG_EOF

# Create README
cat > "$PACKAGE_DIR/README.txt" << 'README_EOF'
Tic Tac Flic Flac Floe - Redistribution Package
================================================

This package contains everything needed to deploy the game to any web server.

QUICK START
-----------

1. Run the deployment script with your target URL:
   
   Linux/Mac:    ./deploy.sh https://your-domain.com/games/ttfff
   Windows:      deploy.bat https://your-domain.com/games/ttfff
   
   For root deployment, use:
   Linux/Mac:    ./deploy.sh /
   Windows:      deploy.bat /

2. Upload all files to your web server

3. Configure your web server (see WEB_SERVER_CONFIG.txt)

4. Visit the URL to play!


LOCAL TESTING
-------------

To test locally before deploying:

Python 3:
    python3 -m http.server 8080
    
Node.js (with 'serve' package):
    npm install -g serve
    serve -s . -l 8080

Then visit: http://localhost:8080


WHAT'S INCLUDED
---------------

index.html              - Main HTML file
manifest.json           - PWA manifest
sw.js                   - Service worker for offline support
*.js, *.wasm           - Compiled application files
deploy.sh              - Deployment script (Linux/Mac)
deploy.bat             - Deployment script (Windows)
WEB_SERVER_CONFIG.txt  - Web server configuration guide
README.txt             - This file


DEPLOYMENT OPTIONS
------------------

Root deployment:
    ./deploy.sh /
    Files will be at: https://example.com/

Subdirectory deployment:
    ./deploy.sh https://example.com/games/ttfff
    Files will be at: https://example.com/games/ttfff/


SUPPORT
-------

For issues or questions:
https://github.com/emberian/tictacflicflacfloe

README_EOF

echo "[4/4] Creating ZIP file..."
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
ZIP_NAME="ttfff-redist-$TIMESTAMP.zip"

cd "$PACKAGE_DIR"
zip -r "../$ZIP_NAME" ./*
cd ..

rm -rf "$PACKAGE_DIR"

echo ""
echo "==============================================="
echo "Redistribution Package Created Successfully!"
echo "==============================================="
echo ""
echo "Package: $ZIP_NAME"
echo ""
echo "This package contains:"
echo "  - Pre-built website files"
echo "  - deploy.sh (Linux/Mac deployment script)"
echo "  - deploy.bat (Windows deployment script)"
echo "  - WEB_SERVER_CONFIG.txt (server configuration guide)"
echo "  - README.txt (instructions)"
echo ""
echo "To use:"
echo "  1. Extract the ZIP file"
echo "  2. Run the appropriate deploy script with target URL"
echo "  3. Upload files to web server"
echo ""
