const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
const scenes = document.querySelectorAll('.scene');
if ('IntersectionObserver' in window && !reducedMotion.matches) {
  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        entry.target.classList.add('in-view');
        observer.unobserve(entry.target);
      }
    }
  }, { threshold: 0.18 });
  scenes.forEach((scene) => observer.observe(scene));
} else {
  scenes.forEach((scene) => scene.classList.add('in-view'));
}
