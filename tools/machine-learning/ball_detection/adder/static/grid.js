const svgNamespace = 'http://www.w3.org/2000/svg';

const updateSvgViewBox = async (svg, imageIndex) => {
  // set previous width and height
  svg.setAttributeNS(null, 'viewBox', `0 0 ${annotationImageSizes[imageIndex].width} ${annotationImageSizes[imageIndex].height}`);

  if (annotationImageSizes[imageIndex].width === 0 && annotationImageSizes[imageIndex].height === 0) {
    // only fetch new image sizes of new images
    const response = await fetch(`/image_size/${imageIndex}`);
    annotationImageSizes[imageIndex] = await response.json();
    svg.setAttributeNS(null, 'viewBox', `0 0 ${annotationImageSizes[imageIndex].width} ${annotationImageSizes[imageIndex].height}`);
  }
};

const updateVisibleImages = entries => {
  entries.forEach(entry => {
    const i = parseInt(entry.target.id);
    if (entry.isIntersecting) {
      const image = annotationImages[i];
      const existingCircles = existingAnnotations[image];
      const addedCircles = addedAnnotations[image];

      const div = document.createElement('div');
      entry.target.appendChild(div);

      const svg = document.createElementNS(svgNamespace, 'svg');
      div.appendChild(svg);
      updateSvgViewBox(svg, i);
      addedCircles.forEach(circle => {
        const rect = document.createElementNS(svgNamespace, 'rect');
        svg.appendChild(rect);
        rect.setAttributeNS(null, 'x', `${circle.centerX - circle.radius}`);
        rect.setAttributeNS(null, 'y', `${circle.centerY - circle.radius}`);
        rect.setAttributeNS(null, 'width', `${2 * circle.radius}`);
        rect.setAttributeNS(null, 'height', `${2 * circle.radius}`);
        if (existingCircles.some(existingCircle => existingCircle.centerX === circle.centerX && existingCircle.centerY === circle.centerY && existingCircle.radius === circle.radius)) {
          rect.setAttributeNS(null, 'fill', `rgb(${config.existingAnnotationColor[0]}, ${config.existingAnnotationColor[1]}, ${config.existingAnnotationColor[2]})`);
        } else {
          rect.setAttributeNS(null, 'fill', `rgb(${config.addedAnnotationColor[0]}, ${config.addedAnnotationColor[1]}, ${config.addedAnnotationColor[2]})`);
        }
      });

      const img = document.createElement('img');
      div.appendChild(img);
      img.style.width = `${config.gridImageSize}px`;
      img.src = `/image/${i}?&scale&width=${config.gridImageSize}`;
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
        'image': annotationImages[i],
        'visible': entry.isIntersecting,
      });
    });
  });
  for (let i = 0; i < annotationImages.length; ++i) {
    const cell = document.createElement('a');
    cell.id = `${i}`;
    cell.href = `#maximize${i}`;
    grid.appendChild(cell);
    observer.observe(cell);
    telemetryObserver.observe(cell);
  }

  window.location.hash = '';
  window.addEventListener('hashchange', () => {
    if (window.location.hash.length > 0) {
      const i = parseInt(window.location.hash.substr(9));
      maximize(document.getElementById(`${i}`));
    } else {
      minimize(() => {
        if (maximizedCellElement !== null) {
          const i = parseInt(maximizedCellElement.id);
          const svg = maximizedCellElement.querySelector('div>svg');
          const image = annotationImages[i];
          const existingCircles = existingAnnotations[image];
          const addedCircles = addedAnnotations[image];

          while (svg.firstChild) {
            svg.removeChild(svg.firstChild);
          }

          addedCircles.forEach(circle => {
            const rect = document.createElementNS(svgNamespace, 'rect');
            svg.appendChild(rect);
            rect.setAttributeNS(null, 'x', `${circle.centerX - circle.radius}`);
            rect.setAttributeNS(null, 'y', `${circle.centerY - circle.radius}`);
            rect.setAttributeNS(null, 'width', `${2 * circle.radius}`);
            rect.setAttributeNS(null, 'height', `${2 * circle.radius}`);
            if (existingCircles.some(existingCircle => existingCircle.centerX === circle.centerX && existingCircle.centerY === circle.centerY && existingCircle.radius === circle.radius)) {
              rect.setAttributeNS(null, 'fill', `rgb(${config.existingAnnotationColor[0]}, ${config.existingAnnotationColor[1]}, ${config.existingAnnotationColor[2]})`);
            } else {
              rect.setAttributeNS(null, 'fill', `rgb(${config.addedAnnotationColor[0]}, ${config.addedAnnotationColor[1]}, ${config.addedAnnotationColor[2]})`);
            }
          });
        }
      });
    }
  });

  window.addEventListener('keypress', event => {
    if (event.key === "s") {
      addTelemetryMessage({
        'timestamp': (new Date).toISOString(),
        'type': 'scrollToLast',
      });
      for (let i = annotationImages.length - 1; i >= 0; --i) {
        const image = annotationImages[i];
        const existingCircles = existingAnnotations[image];
        const addedCircles = addedAnnotations[image];
        for (const circle of addedCircles) {
          if (!existingCircles.some(existingCircle => existingCircle.centerX === circle.centerX && existingCircle.centerY === circle.centerY && existingCircle.radius === circle.radius)) {
            document.getElementById(`${i}`).scrollIntoView();
            return;
          }
        }
      }

      alert('Cannot scroll last added circle into view: There aren\'t any added circles');
    }
  });
};
