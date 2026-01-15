// Utilities to generate the data key and the intermediate keys
const fs = require("fs");

async function generateDataKey() {
  // Generate random data encryption key (DEK)
  const dataKeyRaw = crypto.getRandomValues(new Uint8Array(32));
  const dataKey = await crypto.subtle.importKey(
    "raw",
    dataKeyRaw,
    "AES-GCM",
    false,
    ["encrypt"]
  );

  fs.writeFileSync("crypto/secret_data_key", dataKeyRaw);
}

generateDataKey();
