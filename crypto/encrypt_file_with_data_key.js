const fs = require("fs");
const { webcrypto: crypto } = require("crypto");
const { readDataKey } = require("./util");

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

  const encryptedInfo = {
    iv: btoa(String.fromCharCode(...dataIv)),
    ciphertext: btoa(String.fromCharCode(...new Uint8Array(ciphertext))),
  };

  fs.writeFileSync("encrypted_info.json", JSON.stringify(encryptedInfo));
}

encrypt();
