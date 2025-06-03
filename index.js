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
