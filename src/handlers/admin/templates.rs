//! Admin UI HTML templates

/// Admin dashboard HTML template
pub const ADMIN_DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>rcodex Admin</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; color: #333; }
        .container { max-width: 1200px; margin: 0 auto; padding: 20px; }
        header { background: #1a1a2e; color: white; padding: 20px; margin-bottom: 20px; border-radius: 8px; }
        header h1 { font-size: 1.8em; }
        .card { background: white; border-radius: 8px; padding: 20px; margin-bottom: 20px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .card h2 { margin-bottom: 15px; color: #1a1a2e; border-bottom: 2px solid #eee; padding-bottom: 10px; }
        .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin-bottom: 20px; }
        .stat-card { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; border-radius: 8px; }
        .stat-card h3 { font-size: 0.9em; opacity: 0.9; }
        .stat-card .value { font-size: 2em; font-weight: bold; margin-top: 5px; }
        table { width: 100%; border-collapse: collapse; }
        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #eee; }
        th { background: #f8f9fa; font-weight: 600; }
        .status { display: inline-block; padding: 4px 8px; border-radius: 4px; font-size: 0.85em; }
        .status.healthy { background: #d4edda; color: #155724; }
        .badge { display: inline-block; background: #e9ecef; padding: 2px 8px; border-radius: 12px; font-size: 0.8em; margin: 2px; }
        .refresh { background: #007bff; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; }
        .refresh:hover { background: #0056b3; }
        #status { margin-top: 20px; }
        .error { color: #dc3545; padding: 10px; background: #f8d7da; border-radius: 4px; display: none; }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>rcodex Admin Dashboard</h1>
            <p>OpenAI Proxy Management Interface</p>
        </header>
        
        <div class="stats" id="stats">
            <div class="stat-card">
                <h3>Providers</h3>
                <div class="value" id="provider-count">-</div>
            </div>
            <div class="stat-card" style="background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);">
                <h3>Models</h3>
                <div class="value" id="model-count">-</div>
            </div>
            <div class="stat-card" style="background: linear-gradient(135deg, #fc4a1a 0%, #f7b733 100%);">
                <h3>Uptime</h3>
                <div class="value" id="uptime">-</div>
            </div>
        </div>
        
        <div class="card">
            <h2>System Status</h2>
            <button class="refresh" onclick="loadStatus()">Refresh</button>
            <div id="status"></div>
            <div class="error" id="error"></div>
        </div>
        
        <div class="card">
            <h2>Providers</h2>
            <div id="providers"></div>
        </div>
    </div>
    
    <script>
        async function loadStatus() {
            const errorEl = document.getElementById('error');
            errorEl.style.display = 'none';
            
            try {
                const response = await fetch('/admin/api/status');
                if (!response.ok) throw new Error('Failed to load status');
                
                const data = await response.json();
                
                // Update stats
                document.getElementById('provider-count').textContent = data.providers.length;
                document.getElementById('model-count').textContent = data.providers.reduce((sum, p) => sum + p.models.length, 0);
                document.getElementById('uptime').textContent = formatUptime(data.uptime_seconds);
                
                // Update status
                const statusEl = document.getElementById('status');
                statusEl.innerHTML = `
                    <p><strong>Status:</strong> <span class="status healthy">${data.status}</span></p>
                    <p><strong>Version:</strong> ${data.version}</p>
                `;
                
                // Update providers table
                const providersEl = document.getElementById('providers');
                if (data.providers.length === 0) {
                    providersEl.innerHTML = '<p>No providers configured</p>';
                } else {
                    providersEl.innerHTML = `
                        <table>
                            <thead>
                                <tr>
                                    <th>ID</th>
                                    <th>Name</th>
                                    <th>Base URL</th>
                                    <th>Models</th>
                                    <th>Status</th>
                                </tr>
                            </thead>
                            <tbody>
                                ${data.providers.map(p => `
                                    <tr>
                                        <td>${p.id}</td>
                                        <td>${p.name}</td>
                                        <td>${p.base_url}</td>
                                        <td>${p.models.map(m => `<span class="badge">${m}</span>`).join('')}</td>
                                        <td><span class="status healthy">${p.enabled ? 'Active' : 'Disabled'}</span></td>
                                    </tr>
                                `).join('')}
                            </tbody>
                        </table>
                    `;
                }
            } catch (err) {
                errorEl.textContent = 'Error loading status: ' + err.message;
                errorEl.style.display = 'block';
            }
        }
        
        function formatUptime(seconds) {
            const days = Math.floor(seconds / 86400);
            const hours = Math.floor((seconds % 86400) / 3600);
            const mins = Math.floor((seconds % 3600) / 60);
            if (days > 0) return `${days}d ${hours}h`;
            if (hours > 0) return `${hours}h ${mins}m`;
            return `${mins}m`;
        }
        
        // Load on page load
        loadStatus();
        
        // Auto-refresh every 30 seconds
        setInterval(loadStatus, 30000);
    </script>
</body>
</html>
"#;
