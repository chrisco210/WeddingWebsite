const fs = require("fs");
const { webcrypto: crypto } = require("crypto");
const { readDataKey } = require("./util");

async function encrypt() {
  const inputFile = process.argv[2];
  if (!inputFile) {
    console.error("Usage: node encrypt_file_with_data_key.js <input-file>");
    process.exit(1);
  }

  const plaintext = fs.readFileSync(inputFile, "utf8").trim();
  const outputFile = "website/encrypted_info.json";

  const dataKey = await readDataKey();

  const dataIv = crypto.getRandomValues(new Uint8Array(12));
  const ciphertext = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv: dataIv },
    dataKey,
    new TextEncoder().encode(plaintext),
  );

  const encryptedInfo = {
    iv: btoa(String.fromCharCode(...dataIv)),
    ciphertext: btoa(String.fromCharCode(...new Uint8Array(ciphertext))),
  };

  fs.writeFileSync(outputFile, JSON.stringify(encryptedInfo));
  console.log(`Encrypted ${inputFile} -> ${outputFile}`);
}

encrypt();
