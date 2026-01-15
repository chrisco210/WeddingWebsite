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

async function encrypt() {
  const enc = new TextEncoder();
  const plaintext = fs.readFileSync("moreinfo_plaintext.html");

  const dataKey = await readDataKey();

  console.log("Data key:" + dataKey);

  const dataIv = crypto.getRandomValues(new Uint8Array(12));
  const ciphertext = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv: dataIv },
    dataKey,
    new Uint8Array(plaintext)
  );

  fs.writeFileSync(
    "encrypted_info.bin",
    Buffer.from(new Uint8Array(ciphertext))
  );
}

encrypt();
