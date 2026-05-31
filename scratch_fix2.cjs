const fs = require('fs');
const p = 'g:/oskarlolpo project/minecraftjava/01_Active/p2p/src/main.js';
let content = fs.readFileSync(p, 'utf8');

// Fix 1: The UI rendering logic hardcoding localhost:25565
const target1 = `const displayAddr = s.minecraft_version.includes("Bedrock") ? state.tunnelAddr : "localhost:25565";`;
const repl1 = `const displayAddr = state.tunnelAddr;`;
content = content.replace(target1, repl1);

// Fix 2: The Copy button logic hardcoding localhost:25565
const target2 = `endpoint = "localhost:25565"; // Default proxy port`;
const repl2 = `endpoint = state.tunnelAddr || "localhost:25565"; // Default proxy port`;
content = content.replace(target2, repl2);

fs.writeFileSync(p, content);
console.log('Fixed java endpoints');
