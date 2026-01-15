// Utilities to generate the data key and the intermediate keys
const fs = require("fs");

const outputName = "crypto/secret/secret_data_key";

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

  if (fs.existsSync(outputName)) {
    throw new Error("Data key file already exists: " + outputName);
  }

  fs.writeFileSync(outputName, dataKeyRaw);
}

generateDataKey();
