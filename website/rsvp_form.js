function initRsvpForm() {
  let searchTimer = null;

  const nameInput = document.getElementById("name-search");
  if (!nameInput) return; // not on the RSVP search page

  const params = new URLSearchParams(window.location.search);
  const RSVP_API_BASE = params.get("api_base");
  if (!RSVP_API_BASE) {
    window.location.href = "rsvp.html";
    return;
  }

  const searchResults = document.getElementById("search-results");

  // ── Search ─────────────────────────────────────────────────────────────────

  nameInput.addEventListener("input", () => {
    clearTimeout(searchTimer);
    const q = nameInput.value.trim();
    if (q.length < 2) {
      searchResults.innerHTML = "";
      return;
    }
    searchTimer = setTimeout(() => doSearch(q), 350);
  });

  async function doSearch(query) {
    try {
      const resp = await fetch(
        `${RSVP_API_BASE}/rsvp?search=${encodeURIComponent(query)}`,
      );
      if (!resp.ok) return;
      const data = await resp.json();
      renderResults(data.matches || []);
    } catch (e) {
      console.error("Search error:", e);
    }
  }

  function renderResults(matches) {
    searchResults.innerHTML = "";
    if (matches.length === 0) {
      const li = document.createElement("li");
      li.textContent = "No matches found.";
      li.className = "no-results";
      searchResults.appendChild(li);
      return;
    }
    for (const match of matches) {
      const li = document.createElement("li");
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "search-result-btn";
      btn.innerHTML = `<span class="search-result-text"><span class="search-result-name">${match.display_name}</span><span class="search-result-guests">${match.guest_names.join(", ")}</span></span>`;
      btn.addEventListener("click", () => {
        window.location.href =
          `rsvp_submit.html?party_id=${encodeURIComponent(match.party_id)}` +
          `&api_base=${encodeURIComponent(RSVP_API_BASE)}`;
      });
      li.appendChild(btn);
      searchResults.appendChild(li);
    }
  }
}

window.addEventListener("DOMContentLoaded", initRsvpForm);
