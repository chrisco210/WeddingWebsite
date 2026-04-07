(async function initRsvpSubmit() {
  const params = new URLSearchParams(window.location.search);
  const partyId = params.get("party_id");
  const RSVP_API_BASE = params.get("api_base");

  if (!partyId || !RSVP_API_BASE) {
    window.location.href = "rsvp.html";
    return;
  }

  const partySection = document.getElementById("rsvp-party");
  const partyNameEl = document.getElementById("party-name");
  const guestList = document.getElementById("guest-list");
  const rsvpForm = document.getElementById("rsvp-form");
  const rsvpStatus = document.getElementById("rsvp-status");
  const backBtn = document.getElementById("back-btn");

  // ── Confirmation ───────────────────────────────────────────────────────────

  const confirmationSection = document.createElement("div");
  confirmationSection.id = "rsvp-confirmation";
  confirmationSection.style.display = "none";
  confirmationSection.innerHTML = `
    <div class="confirmation-content">
      <h3>Thank you!</h3>
      <p>We can't wait to celebrate with you!</p>
      <div id="confirmation-summary"></div>
      <button type="button" id="rsvp-again-btn" class="button-link transition-button-link">Update RSVP</button>
    </div>
  `;
  partySection.parentNode.insertBefore(
    confirmationSection,
    partySection.nextSibling,
  );

  confirmationSection
    .querySelector("#rsvp-again-btn")
    .addEventListener("click", () => {
      window.location.href =
        `rsvp_submit.html?party_id=${encodeURIComponent(partyId)}` +
        `&api_base=${encodeURIComponent(RSVP_API_BASE)}`;
    });

  function showConfirmation(responses) {
    const summary = confirmationSection.querySelector("#confirmation-summary");
    summary.innerHTML = responses
      .map((r) => {
        const badgeClass = r.attending ? "rsvp-badge--yes" : "rsvp-badge--no";
        const label = r.attending ? "Attending" : "Not Attending";
        return `
          <div class="guest-card guest-card--readonly">
            <p class="guest-name">${r.name}</p>
            <div class="rsvp-summary">
              <span class="rsvp-badge ${badgeClass}">${label}</span>
            </div>
          </div>`;
      })
      .join("");
    partySection.style.display = "none";
    confirmationSection.style.display = "block";
  }

  // ── Render ─────────────────────────────────────────────────────────────────

  function renderParty(party) {
    partySection.style.display = "block";
    rsvpStatus.textContent = "";

    const hasExistingRsvp = party.guests.some((g) => g.rsvp != null);
    if (hasExistingRsvp) {
      renderReadOnly(party);
    } else {
      renderEditable(party);
    }
  }

  function renderReadOnly(party) {
    partyNameEl.innerHTML = `<h3>${party.display_name}</h3><p class="rsvp-form-title">Your RSVP</p>`;
    guestList.innerHTML = "";

    for (const guest of party.guests) {
      const card = document.createElement("div");
      card.className = "guest-card guest-card--readonly";
      const attending = guest.rsvp?.attending;
      const dietary = guest.rsvp?.dietary_restrictions;

      let statusHtml;
      if (attending === true) {
        statusHtml = `<span class="rsvp-badge rsvp-badge--yes">Attending</span>`;
        if (dietary) {
          statusHtml += `<span class="rsvp-dietary">${dietary}</span>`;
        }
      } else if (attending === false) {
        statusHtml = `<span class="rsvp-badge rsvp-badge--no">Not Attending</span>`;
      } else {
        statusHtml = `<span class="rsvp-badge rsvp-badge--none">No response</span>`;
      }

      card.innerHTML = `
        <p class="guest-name">${guest.name}</p>
        <div class="rsvp-summary">${statusHtml}</div>
      `;
      guestList.appendChild(card);
    }

    rsvpForm.querySelector('[type="submit"]').style.display = "none";
    const editBtn = document.createElement("button");
    editBtn.type = "button";
    editBtn.className = "button-link transition-button-link";
    editBtn.textContent = "Edit RSVP";
    editBtn.addEventListener("click", () => {
      editBtn.remove();
      renderEditable(party);
    });
    rsvpForm.querySelector(".rsvp-actions").prepend(editBtn);
  }

  function renderEditable(party) {
    partyNameEl.innerHTML = `<h3>${party.display_name}</h3><p class="rsvp-form-title">RSVP</p>`;
    guestList.innerHTML = "";

    const submitBtn = rsvpForm.querySelector('[type="submit"]');
    submitBtn.style.display = "";
    submitBtn.disabled = false;

    for (const guest of party.guests) {
      const prevYes = guest.rsvp?.attending === true;
      const prevNo = guest.rsvp?.attending === false;

      const card = document.createElement("div");
      card.className = "guest-card";
      card.dataset.guestName = guest.name;
      card.innerHTML = `
        <p class="guest-name">${guest.name}</p>
        <div class="attend-toggle">
          <button type="button" class="attend-btn${prevYes ? " attend-yes" : ""}" data-attending="true">Attending</button>
          <button type="button" class="attend-btn${prevNo ? " attend-no" : ""}" data-attending="false">Not Attending</button>
        </div>
      `;

      card.querySelectorAll(".attend-btn").forEach((btn) => {
        btn.addEventListener("click", () => {
          card
            .querySelectorAll(".attend-btn")
            .forEach((b) => b.classList.remove("attend-yes", "attend-no"));
          const isYes = btn.dataset.attending === "true";
          btn.classList.add(isYes ? "attend-yes" : "attend-no");
        });
      });

      guestList.appendChild(card);
    }
  }

  // ── Submit ─────────────────────────────────────────────────────────────────

  rsvpForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    rsvpStatus.textContent = "";

    const responses = [];
    for (const card of guestList.querySelectorAll(".guest-card")) {
      const name = card.dataset.guestName;
      const yesSelected = card
        .querySelector('[data-attending="true"]')
        .classList.contains("attend-yes");
      const noSelected = card
        .querySelector('[data-attending="false"]')
        .classList.contains("attend-no");

      if (!yesSelected && !noSelected) {
        rsvpStatus.textContent = `Please select attending or not attending for ${name}.`;
        return;
      }

      responses.push({ name, attending: yesSelected });
    }

    const submitBtn = rsvpForm.querySelector('[type="submit"]');
    submitBtn.disabled = true;

    try {
      const resp = await fetch(`${RSVP_API_BASE}/rsvp`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ party_id: partyId, responses }),
      });
      if (resp.ok) {
        showConfirmation(responses);
      } else {
        rsvpStatus.textContent = `Error: ${await resp.text()}`;
        submitBtn.disabled = false;
      }
    } catch (e) {
      console.error("Submit error:", e);
      rsvpStatus.textContent = "Network error — please try again.";
      submitBtn.disabled = false;
    }
  });

  // ── Back ───────────────────────────────────────────────────────────────────

  backBtn.addEventListener("click", () => {
    window.location.href = "rsvp.html";
  });

  // ── Load ───────────────────────────────────────────────────────────────────

  try {
    const resp = await fetch(
      `${RSVP_API_BASE}/rsvp?party_id=${encodeURIComponent(partyId)}`,
    );
    if (!resp.ok) {
      partySection.innerHTML = "<p>Could not load party details.</p>";
      return;
    }
    renderParty(await resp.json());
  } catch (e) {
    console.error("Load party error:", e);
    partySection.innerHTML = "<p>Network error — please try again.</p>";
  }
})();
