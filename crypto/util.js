const fs = require("fs");
const { webcrypto: crypto } = require("crypto");

async function readDataKeyRaw() {
  const dataKeyRaw = fs.readFileSync("crypto/secret/secret_data_key");
  const dataKey = await crypto.subtle.importKey(
    "raw",
    new Uint8Array(dataKeyRaw),
    "AES-GCM",
    false,
    ["encrypt"]
  );

  return dataKey;
}

function readPasswords() {
  try {
    const content = fs.readFileSync(
      "crypto/secret/passwords_secret.json",
      "utf8"
    );
    const obj = JSON.parse(content);
    return Array.isArray(obj.passwords) ? obj.passwords : [];
  } catch (e) {
    return [];
  }
}

module.exports = { readDataKeyRaw, readPasswords };
