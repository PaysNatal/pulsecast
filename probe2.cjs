const WebSocket = require('ws');
const ws = new WebSocket('ws://localhost:4598');
const got = [];
ws.on('open', () => { setTimeout(()=>{ ws.send(JSON.stringify({kind:'flash'})); }, 400); });
ws.on('message', (d) => { got.push(d.toString()); });
setTimeout(() => { console.log('RECV='+JSON.stringify(got)); process.exit(0); }, 1200);
