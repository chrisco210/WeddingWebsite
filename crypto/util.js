const fs = require("fs");
const { webcrypto: crypto } = require("crypto");

async function readDataKey() {
  const dataKeyRaw = fs.readFileSync("crypto/secret_data_key");
  console.log("Data key raw: " + dataKeyRaw.length);
  const dataKey = await crypto.subtle.importKey(
    "raw",
    new Uint8Array(dataKeyRaw),
    "AES-GCM",
    false,
    ["encrypt"]
  );

  return dataKey;
}

function b64ToBytes(b64) {
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}

module.exports = { readDataKey };
