window.addEventListener("DOMContentLoaded", function () {
  updateCountdown();
  setInterval(updateCountdown, 1000 * 60 * 60);
});

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
  const password = document.getElementById("password").value;
  const output = document.getElementById("output");

  const envelopedKeys = await fetch("enveloped_keys.json").then((r) =>
    r.json()
  );

  console.log(`Found ${envelopedKeys.keys.length} valid keys`);

  const enc = new TextEncoder();

  const keyMaterial = await crypto.subtle.importKey(
    "raw",
    enc.encode(password),
    "PBKDF2",
    false,
    ["deriveKey"]
  );

  let dataKey = null;

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
        false,
        ["decrypt"]
      );

      console.log("Derived intermediate key from password.");

      const rawKey = await crypto.subtle.decrypt(
        { name: "AES-GCM", iv: b64ToBytes(entry.iv) },
        kek,
        b64ToBytes(entry.wrappedKey)
      );

      console.log("Successfully decrypted data key.");

      dataKey = await crypto.subtle.importKey("raw", rawKey, "AES-GCM", false, [
        "decrypt",
      ]);

      console.log("Successfully imported data key.");
      break;
    } catch (e) {
      console.log("Failed to decrypt key entry", e.toString());
      // Wrong password for this entry
    }
  }

  if (!dataKey) {
    console.log("Invalid password.");
    output.textContent = "Unrecognized password.";
    return;
  }

  try {
    console.log("Fouund valid password, decrypting page.");

    const encryptedInfo = await fetch("encrypted_info.json").then((r) =>
      r.json()
    );

    const plaintext = await crypto.subtle.decrypt(
      { name: "AES-GCM", iv: b64ToBytes(encryptedInfo.iv) },
      dataKey,
      b64ToBytes(encryptedInfo.ciphertext)
    );

    const newContent = new TextDecoder().decode(plaintext);

    document.documentElement.innerHTML = newContent;
  } catch (e) {
    console.error("Failed to decrypt page with valid password.", e);
    output.textContent = "Failed to decrypt page. Email me.";
  }
}
