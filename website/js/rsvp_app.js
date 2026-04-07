// ─── Types ────────────────────────────────────────────────────────────────────

/**
 * @typedef {{ attending: boolean, dietaryRestrictions: string | null }} GuestRsvp
 * @typedef {{ name: string, rsvp: GuestRsvp | null }} Guest
 * @typedef {{ partyId: string, displayName: string, guests: Guest[] }} Party
 * @typedef {{ partyId: string, displayName: string, guestNames: string[] }} SearchMatch
 * @typedef {{ name: string, attending: boolean }} RsvpResponse
 *
 * @typedef {{ party: Party, editable: boolean }} PartyFormData
 * @typedef {{ partyId: string, responses: RsvpResponse[] }} ConfirmedData
 * @typedef {PartyFormData | ConfirmedData | null} StateData
 */

// ─── API Layer ────────────────────────────────────────────────────────────────
// Encapsulates HTTP communication and converts raw API responses to data models.

class RsvpApi {
  /** @param {string} apiBase */
  constructor(apiBase) {
    this.apiBase = apiBase;
  }

  /**
   * Search for parties matching a name query.
   * @param {string} query
   * @returns {Promise<SearchMatch[]>}
   */
  async searchGuests(query) {
    const resp = await fetch(
      `${this.apiBase}/rsvp?search=${encodeURIComponent(query)}`,
    );
    if (!resp.ok) throw new Error(`Search failed: ${resp.status}`);
    const data = await resp.json();
    return (data.matches || []).map((m) => ({
      partyId: m.party_id,
      displayName: m.display_name,
      guestNames: m.guest_names,
    }));
  }

  /**
   * Fetch the full party record for a given party ID.
   * @param {string} partyId
   * @returns {Promise<Party>}
   */
  async getParty(partyId) {
    const resp = await fetch(
      `${this.apiBase}/rsvp?party_id=${encodeURIComponent(partyId)}`,
    );
    if (!resp.ok) throw new Error(`Could not load party: ${resp.status}`);
    const data = await resp.json();
    return {
      partyId: data.party_id,
      displayName: data.display_name,
      guests: data.guests.map((g) => ({
        name: g.name,
        rsvp: g.rsvp
          ? {
              attending: g.rsvp.attending,
              dietaryRestrictions: g.rsvp.dietary_restrictions,
            }
          : null,
      })),
    };
  }

  /**
   * Submit attendance responses for a party.
   * @param {string} partyId
   * @param {RsvpResponse[]} responses
   * @returns {Promise<void>}
   */
  async submitRsvp(partyId, responses) {
    const resp = await fetch(`${this.apiBase}/rsvp`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        party_id: partyId,
        responses: responses.map((r) => ({
          name: r.name,
          attending: r.attending,
        })),
      }),
    });
    if (!resp.ok) throw new Error(await resp.text());
  }
}

// ─── Model ────────────────────────────────────────────────────────────────────
// State machine representing which page is shown and the data it needs.

/** @enum {string} */
const State = Object.freeze({
  SEARCH: "SEARCH",
  PARTY_FORM: "PARTY_FORM",
  CONFIRMED: "CONFIRMED",
});

class RsvpModel {
  constructor() {
    /** @type {State[keyof State]} */
    this.state = State.SEARCH;
    /** @type {StateData} */
    this.data = null;
    /** @type {Array<(state: State[keyof State], data: StateData) => void>} */
    this._listeners = [];
  }

  /**
   * Move to a new state and notify all listeners.
   * @param {State[keyof State]} state
   * @param {StateData} [data]
   */
  transition(state, data = null) {
    this.state = state;
    this.data = data;
    for (const fn of this._listeners) fn(state, data);
  }

  /**
   * Register a callback that fires on every state transition.
   * @param {(state: State[keyof State], data: StateData) => void} fn
   */
  onChange(fn) {
    this._listeners.push(fn);
  }
}

// ─── View ─────────────────────────────────────────────────────────────────────
// Pure DOM manipulation — no business logic, no API calls.

class RsvpView {
  constructor() {
    this.searchView = document.getElementById("view-search");
    this.partyView = document.getElementById("view-party");
    this.confirmView = document.getElementById("view-confirmation");
    this.nameInput = document.getElementById("name-search");
    this.searchResults = document.getElementById("search-results");
    this.partyName = document.getElementById("party-name");
    this.guestList = document.getElementById("guest-list");
    this.rsvpForm = document.getElementById("rsvp-form");
    this.rsvpStatus = document.getElementById("rsvp-status");
    this.submitBtn = document.getElementById("submit-btn");
    this.confirmSummary = document.getElementById("confirmation-summary");
  }

  // ── Search ──────────────────────────────────────────────────────────────────

  /** Show the name-search form, clearing any previous input and results. */
  renderSearch() {
    this._showOnly(this.searchView);
    this.nameInput.value = "";
    this.searchResults.innerHTML = "";
  }

  /**
   * Populate the search results list.
   * @param {SearchMatch[]} matches
   * @param {(partyId: string) => void} onSelect  Called when the user picks a result.
   */
  renderSearchResults(matches, onSelect) {
    this.searchResults.innerHTML = "";
    if (matches.length === 0) {
      const li = document.createElement("li");
      li.textContent = "No matches found.";
      li.className = "no-results";
      this.searchResults.appendChild(li);
      return;
    }
    for (const match of matches) {
      const li = document.createElement("li");
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "search-result-btn";
      btn.innerHTML = `
        <span class="search-result-text">
          <span class="search-result-name">${match.displayName}</span>
          <span class="search-result-guests">${match.guestNames.join(", ")}</span>
        </span>`;
      btn.addEventListener("click", () => onSelect(match.partyId));
      li.appendChild(btn);
      this.searchResults.appendChild(li);
    }
  }

  // ── Party form ──────────────────────────────────────────────────────────────

  /** Show the party view with a loading indicator while the party is being fetched. */
  renderPartyLoading() {
    this._showOnly(this.partyView);
    this.partyName.innerHTML = "<p>Loading…</p>";
    this.guestList.innerHTML = "";
    this.rsvpStatus.textContent = "";
    this.submitBtn.style.display = "none";
    this._removeEditBtn();
  }

  /**
   * Replace the loading indicator with an error message.
   * The back button remains accessible so the user can return to search.
   * @param {string} message
   */
  renderPartyError(message) {
    this.partyName.innerHTML = "";
    this.guestList.innerHTML = `<p>${message}</p>`;
    this.submitBtn.style.display = "none";
    this._removeEditBtn();
  }

  /**
   * Render the party's existing RSVP responses in a non-editable layout.
   * @param {Party} party
   * @param {() => void} onEditClick  Called when the user clicks "Edit RSVP".
   */
  renderPartyReadOnly(party, onEditClick) {
    this._showOnly(this.partyView);
    this.partyName.innerHTML = `<h3>${party.displayName}</h3><p class="rsvp-form-title">Your RSVP</p>`;
    this.rsvpStatus.textContent = "";
    this.guestList.innerHTML = "";
    this.submitBtn.style.display = "none";
    this._removeEditBtn();

    for (const guest of party.guests) {
      this.guestList.appendChild(this._buildReadOnlyCard(guest));
    }

    const editBtn = document.createElement("button");
    editBtn.type = "button";
    editBtn.id = "edit-rsvp-btn";
    editBtn.className = "button-link transition-button-link";
    editBtn.textContent = "Edit RSVP";
    editBtn.addEventListener("click", onEditClick);
    this.rsvpForm.querySelector(".rsvp-actions").prepend(editBtn);
  }

  /**
   * Render the party form with interactive attendance toggles for each guest.
   * @param {Party} party
   * @param {{ [guestName: string]: boolean | null }} selections  Current attendance selections.
   * @param {(guestName: string, isYes: boolean) => void} onToggle  Called on each toggle.
   */
  renderPartyEditable(party, selections, onToggle) {
    this._showOnly(this.partyView);
    this.partyName.innerHTML = `<h3>${party.displayName}</h3><p class="rsvp-form-title">RSVP</p>`;
    this.rsvpStatus.textContent = "";
    this.guestList.innerHTML = "";
    this.submitBtn.style.display = "";
    this.submitBtn.disabled = false;
    this._removeEditBtn();

    for (const guest of party.guests) {
      this.guestList.appendChild(
        this._buildEditableCard(guest, selections[guest.name], onToggle),
      );
    }
  }

  /**
   * Enable or disable the submit button during an in-flight submission.
   * @param {boolean} isSubmitting
   */
  setSubmitting(isSubmitting) {
    this.submitBtn.disabled = isSubmitting;
  }

  /**
   * Display a validation or error message below the guest list.
   * @param {string} message
   */
  setStatusMessage(message) {
    this.rsvpStatus.textContent = message;
  }

  // ── Confirmation ─────────────────────────────────────────────────────────────

  /**
   * Show the post-submission thank-you screen with a summary of responses.
   * @param {RsvpResponse[]} responses
   */
  renderConfirmation(responses) {
    this._showOnly(this.confirmView);
    this.confirmSummary.innerHTML = responses
      .map((r) => {
        const cls = r.attending ? "rsvp-badge--yes" : "rsvp-badge--no";
        const label = r.attending ? "Attending" : "Not Attending";
        return `
          <div class="guest-card guest-card--readonly">
            <p class="guest-name">${r.name}</p>
            <div class="rsvp-summary">
              <span class="rsvp-badge ${cls}">${label}</span>
            </div>
          </div>`;
      })
      .join("");
  }

  // ── Private helpers ──────────────────────────────────────────────────────────

  /** @param {HTMLElement} view */
  _showOnly(view) {
    for (const v of [this.searchView, this.partyView, this.confirmView]) {
      v.style.display = "none";
    }
    view.style.display = "block";
  }

  _removeEditBtn() {
    document.getElementById("edit-rsvp-btn")?.remove();
  }

  /**
   * @param {Guest} guest
   * @returns {HTMLElement}
   */
  _buildReadOnlyCard(guest) {
    const card = document.createElement("div");
    card.className = "guest-card guest-card--readonly";
    const attending = guest.rsvp?.attending;
    const dietary = guest.rsvp?.dietaryRestrictions;

    let statusHtml;
    if (attending === true) {
      statusHtml = `<span class="rsvp-badge rsvp-badge--yes">Attending</span>`;
      if (dietary) statusHtml += `<span class="rsvp-dietary">${dietary}</span>`;
    } else if (attending === false) {
      statusHtml = `<span class="rsvp-badge rsvp-badge--no">Not Attending</span>`;
    } else {
      statusHtml = `<span class="rsvp-badge rsvp-badge--none">No response</span>`;
    }

    card.innerHTML = `
      <p class="guest-name">${guest.name}</p>
      <div class="rsvp-summary">${statusHtml}</div>`;
    return card;
  }

  /**
   * @param {Guest} guest
   * @param {boolean | null} selectedAttending
   * @param {(guestName: string, isYes: boolean) => void} onToggle
   * @returns {HTMLElement}
   */
  _buildEditableCard(guest, selectedAttending, onToggle) {
    const card = document.createElement("div");
    card.className = "guest-card";

    const yesClass = selectedAttending === true ? " attend-yes" : "";
    const noClass = selectedAttending === false ? " attend-no" : "";

    card.innerHTML = `
      <p class="guest-name">${guest.name}</p>
      <div class="attend-toggle">
        <button type="button" class="attend-btn${yesClass}" data-attending="true">Attending</button>
        <button type="button" class="attend-btn${noClass}" data-attending="false">Not Attending</button>
      </div>`;

    card.querySelectorAll(".attend-btn").forEach((btn) => {
      btn.addEventListener("click", () => {
        card
          .querySelectorAll(".attend-btn")
          .forEach((b) => b.classList.remove("attend-yes", "attend-no"));
        const isYes = btn.dataset.attending === "true";
        btn.classList.add(isYes ? "attend-yes" : "attend-no");
        onToggle(guest.name, isYes);
      });
    });

    return card;
  }
}

// ─── Controller ───────────────────────────────────────────────────────────────
// Handles user interactions, orchestrates model transitions and API calls.

class RsvpController {
  /**
   * @param {RsvpApi} api
   * @param {RsvpModel} model
   * @param {RsvpView} view
   */
  constructor(api, model, view) {
    this.api = api;
    this.model = model;
    this.view = view;
    /** @type {ReturnType<typeof setTimeout> | null} */
    this._searchTimer = null;
    /** @type {number} Incremented on each new party load to cancel stale responses. */
    this._pendingRequestId = 0;
    /** @type {{ [guestName: string]: boolean | null }} */
    this._selections = {};
  }

  /** Attach DOM event listeners and render the initial search state. */
  init() {
    this.model.onChange((state, data) => this._render(state, data));

    document
      .getElementById("name-search")
      .addEventListener("input", (e) =>
        this.onSearchInput(e.target.value.trim()),
      );

    document.getElementById("rsvp-form").addEventListener("submit", (e) => {
      e.preventDefault();
      this.onFormSubmit();
    });

    document
      .getElementById("back-btn")
      .addEventListener("click", () => this.onBackClick());

    document
      .getElementById("update-rsvp-btn")
      .addEventListener("click", () => this.onUpdateRsvpClick());

    this.model.transition(State.SEARCH);
  }

  // ── Search ──────────────────────────────────────────────────────────────────

  /**
   * Debounce search input and trigger a guest search after a short delay.
   * @param {string} query
   */
  onSearchInput(query) {
    clearTimeout(this._searchTimer);
    if (query.length < 2) {
      this.view.searchResults.innerHTML = "";
      return;
    }
    this._searchTimer = setTimeout(() => this._doSearch(query), 350);
  }

  // ── Party selection ──────────────────────────────────────────────────────────

  /**
   * Load a party by ID and transition to the party form.
   * Shows the party in read-only mode if an existing RSVP is found,
   * or in editable mode if no RSVP has been submitted yet.
   * @param {string} partyId
   */
  async onPartySelect(partyId) {
    const requestId = ++this._pendingRequestId;
    this.view.renderPartyLoading();
    try {
      const party = await this.api.getParty(partyId);
      if (requestId !== this._pendingRequestId) return;
      const hasExistingRsvp = party.guests.some((g) => g.rsvp != null);
      this._initSelections(party);
      this.model.transition(State.PARTY_FORM, {
        party,
        editable: !hasExistingRsvp,
      });
    } catch (e) {
      if (requestId !== this._pendingRequestId) return;
      console.error("Load party error:", e);
      this.view.renderPartyError(
        "Could not load party details — please try again.",
      );
    }
  }

  // ── Party form ──────────────────────────────────────────────────────────────

  /**
   * Switch the party form from read-only to editable mode.
   * @param {Party} party
   */
  onEditClick(party) {
    this.model.transition(State.PARTY_FORM, { party, editable: true });
  }

  /**
   * Record an attendance toggle from the view into the controller's selection state.
   * @param {string} guestName
   * @param {boolean} isYes
   */
  onAttendToggle(guestName, isYes) {
    this._selections[guestName] = isYes;
  }

  /**
   * Validate selections, then submit the RSVP and transition to the confirmation state.
   * Re-enables the submit button and shows an error message if the request fails.
   */
  async onFormSubmit() {
    const { party } = /** @type {PartyFormData} */ (this.model.data);
    const responses = [];

    for (const guest of party.guests) {
      const attending = this._selections[guest.name];
      if (attending === null || attending === undefined) {
        this.view.setStatusMessage(
          `Please select attending or not attending for ${guest.name}.`,
        );
        return;
      }
      responses.push({ name: guest.name, attending });
    }

    this.view.setSubmitting(true);

    try {
      await this.api.submitRsvp(party.partyId, responses);
      this.model.transition(State.CONFIRMED, {
        partyId: party.partyId,
        responses,
      });
    } catch (e) {
      console.error("Submit error:", e);
      this.view.setStatusMessage(
        e.message || "Network error — please try again.",
      );
      this.view.setSubmitting(false);
    }
  }

  // ── Confirmation ─────────────────────────────────────────────────────────────

  /** Re-fetch the party and return to the editable form to allow updating the RSVP. */
  onUpdateRsvpClick() {
    const { partyId } = /** @type {ConfirmedData} */ (this.model.data);
    this.onPartySelect(partyId);
  }

  // ── Navigation ───────────────────────────────────────────────────────────────

  /** Cancel any in-flight party load and return to the search view. */
  onBackClick() {
    this._pendingRequestId++;
    this.model.transition(State.SEARCH);
  }

  // ── Private helpers ──────────────────────────────────────────────────────────

  /** @param {string} query */
  async _doSearch(query) {
    try {
      const matches = await this.api.searchGuests(query);
      this.view.renderSearchResults(matches, (partyId) =>
        this.onPartySelect(partyId),
      );
    } catch (e) {
      console.error("Search error:", e);
    }
  }

  /**
   * Seed `_selections` from a freshly loaded party's existing RSVP data.
   * Guests with no prior response are initialized to `null`.
   * @param {Party} party
   */
  _initSelections(party) {
    this._selections = {};
    for (const guest of party.guests) {
      this._selections[guest.name] = guest.rsvp?.attending ?? null;
    }
  }

  /**
   * @param {State[keyof State]} state
   * @param {StateData} data
   */
  _render(state, data) {
    switch (state) {
      case State.SEARCH:
        this.view.renderSearch();
        break;
      case State.PARTY_FORM: {
        const { party, editable } = /** @type {PartyFormData} */ (data);
        if (editable) {
          this.view.renderPartyEditable(
            party,
            this._selections,
            (name, isYes) => this.onAttendToggle(name, isYes),
          );
        } else {
          this.view.renderPartyReadOnly(party, () => this.onEditClick(party));
        }
        break;
      }
      case State.CONFIRMED: {
        const { responses } = /** @type {ConfirmedData} */ (data);
        this.view.renderConfirmation(responses);
        break;
      }
    }
  }
}

// ─── Bootstrap ────────────────────────────────────────────────────────────────

const getSHA256Hash = async (input) => {
  const textAsBuffer = new TextEncoder().encode(input);
  const hashBuffer = await window.crypto.subtle.digest("SHA-256", textAsBuffer);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hash = hashArray
    .map((item) => item.toString(16).padStart(2, "0"))
    .join("");
  return hash;
};

const expectedApiUrlHash =
  "a79b45fe6f111e42269fd2e5373a2cfa54f3e09825f2c81347d1e22037d644e3";

window.addEventListener("DOMContentLoaded", async () => {
  const params = new URLSearchParams(window.location.search);
  const apiBase = params.get("api_base");

  const hash = await getSHA256Hash(apiBase);

  if (!apiBase || hash != expectedApiUrlHash) {
    console.log(`API URL not recognized: ${apiBase}`);
    window.location.href = "rsvp.html";
    return;
  }

  const api = new RsvpApi(apiBase);
  const model = new RsvpModel();
  const view = new RsvpView();
  const controller = new RsvpController(api, model, view);
  controller.init();
});
