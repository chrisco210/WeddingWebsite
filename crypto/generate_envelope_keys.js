const { readPasswords, readDataKeyRaw } = require("./util");
const fs = require("fs");

const KDF_ITERATIONS = 150000;

async function generateEnvelopeKeys(dataKeyRaw, passwords) {
  const enc = new TextEncoder();

  const keys = [];
  for (const password of passwords) {
    console.log("Generating envelope key for password: " + password);
    const salt = crypto.getRandomValues(new Uint8Array(16));
    const iv = crypto.getRandomValues(new Uint8Array(12));

    const keyMaterial = await crypto.subtle.importKey(
      "raw",
      enc.encode(password),
      "PBKDF2",
      false,
      ["deriveKey"]
    );

    const kek = await crypto.subtle.deriveKey(
      {
        name: "PBKDF2",
        salt,
        iterations: KDF_ITERATIONS,
        hash: "SHA-256",
      },
      keyMaterial,
      { name: "AES-GCM", length: 256 },
      false,
      ["encrypt"]
    );

    const wrappedKey = await crypto.subtle.encrypt(
      { name: "AES-GCM", iv },
      kek,
      new Uint8Array(dataKeyRaw)
    );

    keys.push({
      salt: btoa(String.fromCharCode(...salt)),
      iv: btoa(String.fromCharCode(...iv)),
      wrappedKey: btoa(String.fromCharCode(...new Uint8Array(wrappedKey))),
    });
  }

  return {
    iterations: KDF_ITERATIONS,
    keys,
  };
}

(async () => {
  const dataKeyRaw = readDataKeyRaw();
  const passwords = readPasswords();

  const envelopeKeys = await generateEnvelopeKeys(dataKeyRaw, passwords);

  fs.writeFileSync("enveloped_keys.json", JSON.stringify(envelopeKeys));
})();
