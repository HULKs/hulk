const svgNamespace = 'http://www.w3.org/2000/svg';

const updateVisibleImages = entries => {
  entries.forEach(entry => {
    const i = parseInt(entry.target.id);
    if (entry.isIntersecting) {
      const div = document.createElement('div');
      entry.target.appendChild(div);

      const svg = document.createElementNS(svgNamespace, 'svg');
      div.appendChild(svg);

      (async () => {
        const response = await fetch(`/annotation/${i}`);
        const annotation = await response.json();
        svg.setAttributeNS(null, 'viewBox', `0 0 ${annotation.dataShape.width} ${annotation.dataShape.height}`);
        if (annotation.isPositive === 1) {
          circleElement = document.createElementNS(svgNamespace, 'circle');
          svg.appendChild(circleElement);
          circleElement.setAttributeNS(null, 'cx', `${annotation.circle.centerX}`);
          circleElement.setAttributeNS(null, 'cy', `${annotation.circle.centerY}`);
          circleElement.setAttributeNS(null, 'r', `${annotation.circle.radius}`);
          circleElement.setAttributeNS(null, 'fill', 'none');
          circleElement.setAttributeNS(null, 'stroke', '#fff');
          circleElement.setAttributeNS(null, 'stroke-width', `${1 / config.gridImageSize * Math.max(annotation.dataShape.width, annotation.dataShape.height)}`);
        }
      })();

      const img = document.createElement('img');
      div.appendChild(img);
      img.style.width = `${config.gridImageSize}px`;
      img.src = `/sample/${i}?scale&width=${config.gridImageSize}&height=${config.gridImageSize}`;
    } else {
      while (entry.target.firstChild) {
        entry.target.removeChild(entry.target.firstChild);
      }
    }
  });
};

const setupGrid = () => {
  const grid = document.querySelector('#grid');
  grid.style.gridTemplateColumns = `repeat(auto-fill, ${config.gridImageSize}px)`;
  grid.style.gridAutoRows = `${config.gridImageSize}px`;

  const observer = new IntersectionObserver(updateVisibleImages, {
    rootMargin: '100% 0% 100% 0%',
    threshold: 0,
  });
  for (let i = 0; i < amountOfSamples; ++i) {
    const cell = document.createElement('div');
    cell.id = `${i}`;
    grid.appendChild(cell);
    observer.observe(cell);
  }
};
