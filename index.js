window.addEventListener("DOMContentLoaded", async function () {
  console.log("DOMContentLoaded");
  updateCountdown();
  setInterval(updateCountdown, 1000 * 60 * 60);

  console.log("Attempting to unlock from local storage...");
  const { envelopedKeys, encryptedInfo } = await retrieveData();

  const result = await tryFromLocalStorage(envelopedKeys);

  if (result) {
    await updateDOM(result.dek, encryptedInfo);
  }
});

const SAVED_KEK_KEY = "saved_kek_result";
const SAVED_IV_KEY = "saved_kek_iv";

const weddingDate = new Date("2026-08-15T00:00:00");

function updateCountdown() {
  const countdownEl = document.getElementById("countdown");
  if (!countdownEl) return;
  console.log("Updating countdown...");
  const now = new Date();
  const diff = weddingDate - now;
  const days = Math.ceil(diff / (1000 * 60 * 60 * 24));
  if (days > 1) {
    countdownEl.textContent = `${days} days to go!`;
  } else if (days === 1) {
    countdownEl.textContent = "1 day to go!";
  } else if (days === 0) {
    countdownEl.textContent = "Today is the day!";
  } else {
    countdownEl.textContent = "The wedding has passed!";
  }
}

function b64ToBytes(b64) {
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}

async function unlock() {
  const { envelopedKeys, encryptedInfo } = await retrieveData();

  const password = document.getElementById("password").value;
  const output = document.getElementById("output");

  const result = await tryPasswords(password, envelopedKeys);

  if (!result) {
    console.log("Invalid password.");
    output.textContent = "Unrecognized password.";
    return;
  }

  const exportKek = await crypto.subtle.exportKey("raw", result.kek);
  const exportKekb64 = btoa(String.fromCharCode(...new Uint8Array(exportKek)));

  if (document.getElementById("remember").checked) {
    localStorage.setItem(SAVED_KEK_KEY, exportKekb64);
    localStorage.setItem(SAVED_IV_KEY, result.iv);
  }

  await updateDOM(result.dek, encryptedInfo);
}

async function retrieveData() {
  const envelopedKeys = await fetch("enveloped_keys.json").then((r) =>
    r.json(),
  );
  const encryptedInfo = await fetch("encrypted_info.json").then((r) =>
    r.json(),
  );
  return { envelopedKeys, encryptedInfo };
}

async function tryFromLocalStorage(envelopedKeys) {
  const savedKekB64 = localStorage.getItem(SAVED_KEK_KEY);
  const savedIvB64 = localStorage.getItem(SAVED_IV_KEY);

  if (!savedKekB64 || !savedIvB64) {
    console.log("No saved KEK in local storage.");
    return null;
  }

  console.log("Trying saved KEK from local storage.");

  const possibleWrappedKey = envelopedKeys.keys.find(
    (entry) => entry.iv === savedIvB64,
  );

  if (!possibleWrappedKey) {
    console.log("No matching wrapped key for saved IV.");
    return null;
  }

  const rawKek = b64ToBytes(savedKekB64);

  const kek = await crypto.subtle.importKey(
    "raw",
    rawKek,
    { name: "AES-GCM" },
    false,
    ["decrypt"],
  );

  try {
    const dekRaw = await crypto.subtle.decrypt(
      { name: "AES-GCM", iv: b64ToBytes(possibleWrappedKey.iv) },
      kek,
      b64ToBytes(possibleWrappedKey.wrappedKey),
    );

    const dek = await crypto.subtle.importKey("raw", dekRaw, "AES-GCM", false, [
      "decrypt",
    ]);

    console.log("Unlocked using stored KEK");
    return { dek };
  } catch (e) {
    // Failed to decrypt, invalid kek
    console.error("Failed to decrypt dek using stored KEK.", e);
    return null;
  }
}

async function tryPasswords(password, envelopedKeys) {
  console.log(`Found ${envelopedKeys.keys.length} valid keys`);

  const enc = new TextEncoder();

  const keyMaterial = await crypto.subtle.importKey(
    "raw",
    enc.encode(password),
    "PBKDF2",
    false,
    ["deriveKey"],
  );

  for (const entry of envelopedKeys.keys) {
    console.log("Attempting to decrypt key " + JSON.stringify(entry));
    try {
      const kek = await crypto.subtle.deriveKey(
        {
          name: "PBKDF2",
          salt: b64ToBytes(entry.salt),
          iterations: envelopedKeys.iterations,
          hash: "SHA-256",
        },
        keyMaterial,
        { name: "AES-GCM", length: 256 },
        true,
        ["decrypt"],
      );

      console.log("Derived intermediate key from password.");

      const dekRaw = await crypto.subtle.decrypt(
        { name: "AES-GCM", iv: b64ToBytes(entry.iv) },
        kek,
        b64ToBytes(entry.wrappedKey),
      );

      const dek = await crypto.subtle.importKey(
        "raw",
        dekRaw,
        "AES-GCM",
        false,
        ["decrypt"],
      );

      console.log("Successfully decrypted data key.");
      return { dek, kek, iv: entry.iv };
    } catch (e) {
      console.log("Failed to decrypt key entry", e.toString());
    }
  }

  return null;
}

async function decryptDataKey(kek, entry) {
  const rawKey = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: b64ToBytes(entry.iv) },
    kek,
    b64ToBytes(entry.wrappedKey),
  );

  const dataKey = await crypto.subtle.importKey(
    "raw",
    rawKey,
    "AES-GCM",
    false,
    ["decrypt"],
  );

  console.log("Successfully imported data key.");
  return dataKey;
}

async function updateDOM(dataKey, encryptedInfo) {
  try {
    console.log("Found valid password, decrypting page.");

    const plaintext = await crypto.subtle.decrypt(
      { name: "AES-GCM", iv: b64ToBytes(encryptedInfo.iv) },
      dataKey,
      b64ToBytes(encryptedInfo.ciphertext),
    );

    const newContent = new DOMParser().parseFromString(
      new TextDecoder().decode(plaintext),
      "text/html",
    );

    document.body = newContent.body;

    updateCountdown();
  } catch (e) {
    console.error("Failed to decrypt page with valid password.", e);
    document.getElementById("output").textContent =
      "Failed to decrypt page. Email me.";
  }
}
