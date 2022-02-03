const svgNamespace = 'http://www.w3.org/2000/svg';

const updateVisibleImages = entries => {
  entries.forEach(entry => {
    const i = parseInt(entry.target.id);
    if (entry.isIntersecting) {
      const circle = annotationCircles[i];

      const div = document.createElement('div');
      entry.target.appendChild(div);

      const svg = document.createElementNS(svgNamespace, 'svg');
      div.appendChild(svg);
      svg.setAttributeNS(null, 'viewBox', `0 0 1 1`);
      if (circle.removed) {
        svg.classList.add('removed');
      }
      const line0 = document.createElementNS(svgNamespace, 'line');
      svg.appendChild(line0);
      line0.setAttributeNS(null, 'x1', '0');
      line0.setAttributeNS(null, 'y1', '0');
      line0.setAttributeNS(null, 'x2', '1');
      line0.setAttributeNS(null, 'y2', '1');
      line0.setAttributeNS(null, 'stroke', '#f00');
      line0.setAttributeNS(null, 'stroke-width', '0.1');
      const line1 = document.createElementNS(svgNamespace, 'line');
      svg.appendChild(line1);
      line1.setAttributeNS(null, 'x1', '1');
      line1.setAttributeNS(null, 'y1', '0');
      line1.setAttributeNS(null, 'x2', '0');
      line1.setAttributeNS(null, 'y2', '1');
      line1.setAttributeNS(null, 'stroke', '#f00');
      line1.setAttributeNS(null, 'stroke-width', '0.1');

      let mouseDownTimeout = null;
      svg.addEventListener('mousedown', () => {
        mouseDownTimeout = setTimeout(() => {
          mouseDownTimeout = null;
          maximize(entry.target);
        }, 200);
      });
      svg.addEventListener('mouseup', async () => {
        if (mouseDownTimeout !== null) {
          clearTimeout(mouseDownTimeout);
          mouseDownTimeout = null;
          const response = await fetch(`/toggle_removed/${i}`);
          const data = await response.json();
          annotationCircles[i].removed = data.removed;
          addTelemetryMessage({
            'timestamp': (new Date).toISOString(),
            'type': 'toggleRemoved',
            'cellId': i,
            'image': circle.image,
            'removed': data.removed,
          });
          if (data.removed) {
            svg.classList.add('removed');
          } else {
            svg.classList.remove('removed');
          }
        } else {
          minimize();
        }
      });

      const img = document.createElement('img');
      div.appendChild(img);
      img.style.width = `${config.gridImageSize}px`;
      img.src = `/image/${circle.imageIndex}?crop&centerX=${circle.centerX}&centerY=${circle.centerY}&radius=${circle.radius * config.cropScaleFactor}&scale&width=${config.gridImageSize}&height=${config.gridImageSize}`;
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
  const telemetryObserver = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      const i = parseInt(entry.target.id);
      addTelemetryMessage({
        'timestamp': (new Date).toISOString(),
        'type': 'cellVisibility',
        'cellId': i,
        'image': annotationCircles[i].image,
        'visible': entry.isIntersecting,
      });
    });
  });
  for (let i = 0; i < annotationCircles.length; ++i) {
    const cell = document.createElement('div');
    cell.id = `${i}`;
    grid.appendChild(cell);
    observer.observe(cell);
    telemetryObserver.observe(cell);
  }

  document.querySelector('#box').addEventListener('mouseup', async () => {
    minimize();
  });

  window.addEventListener('keypress', event => {
    if (event.key === "s") {
      addTelemetryMessage({
        'timestamp': (new Date).toISOString(),
        'type': 'scrollToLast',
      });
      for (let i = annotationCircles.length - 1; i >= 0; --i) {
        const circle = annotationCircles[i];
        if (circle.removed) {
          document.getElementById(`${i}`).scrollIntoView();
          return;
        }
      }

      alert('Cannot scroll last removed circle into view: There aren\'t any removed circles');
    }
  });
};
