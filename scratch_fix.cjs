const fs = require('fs');
const p = 'g:/oskarlolpo project/minecraftjava/01_Active/p2p/src/main.js';
let content = fs.readFileSync(p, 'utf8');

const target = 'roomNameEl.dataset.maxPlayers = "8"; // Default bedrock max players';
const replacement = `try {
        const bedrockName = await invoke("get_latest_bedrock_world_name_command");
        if (bedrockName) {
          roomNameEl.value = bedrockName;
          roomNameEl.dataset.autofilled = "true";
          roomNameEl.dataset.maxPlayers = "8";
          return;
        }
      } catch (e) {
        console.warn("Failed to auto-detect Bedrock world name:", e);
      }
      ` + target;

content = content.replace(target, replacement);
fs.writeFileSync(p, content);
console.log('Fixed desktop main.js');
