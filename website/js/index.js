window.addEventListener("DOMContentLoaded", async function () {
  console.log("DOMContentLoaded");
  updateCountdown();
  setInterval(updateCountdown, 1000 * 60 * 60);
  initGalleryLazyLoad();
});

function initGalleryLazyLoad() {
  const imgs = document.querySelectorAll("img.lazy[data-src]");
  if (!imgs.length) return;

  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const img = entry.target;
          img.src = img.dataset.src;
          img.addEventListener("load", () => img.classList.add("loaded"), { once: true });
          observer.unobserve(img);
        }
      });
    },
    { rootMargin: "200px" }
  );

  imgs.forEach((img) => observer.observe(img));
  initLightbox();
}

function initLightbox() {
  const overlay = document.createElement("div");
  overlay.id = "lightbox-overlay";

  const img = document.createElement("img");
  img.id = "lightbox-img";
  overlay.appendChild(img);

  document.body.appendChild(overlay);

  document.querySelectorAll(".gallery-img").forEach((galleryImg) => {
    galleryImg.style.cursor = "pointer";
    galleryImg.addEventListener("click", () => {
      img.src = galleryImg.src || galleryImg.dataset.src;
      overlay.classList.add("active");
    });
  });

  overlay.addEventListener("click", () => overlay.classList.remove("active"));

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") overlay.classList.remove("active");
  });
}

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
