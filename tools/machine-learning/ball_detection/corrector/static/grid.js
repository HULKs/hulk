const svgNamespace = 'http://www.w3.org/2000/svg';

const createTransparentCircle = (i, svg, circle) => {
  const defs = document.createElementNS(svgNamespace, 'defs');
  svg.appendChild(defs);
  svg.setAttributeNS(null, 'viewBox', `${circle.centerX - (circle.radius * config.cropScaleFactor)} ${circle.centerY - (circle.radius * config.cropScaleFactor)} ${2 * circle.radius * config.cropScaleFactor} ${2 * circle.radius * config.cropScaleFactor}`);
  const mask = document.createElementNS(svgNamespace, 'mask');
  defs.appendChild(mask);
  mask.id = `remove-radius-circle-${i}`;
  const maskRect = document.createElementNS(svgNamespace, 'rect');
  mask.appendChild(maskRect);
  maskRect.setAttributeNS(null, 'x', `${circle.centerX - (circle.radius * config.cropScaleFactor)}`);
  maskRect.setAttributeNS(null, 'y', `${circle.centerY - (circle.radius * config.cropScaleFactor)}`);
  maskRect.setAttributeNS(null, 'width', `${2 * circle.radius * config.cropScaleFactor}`);
  maskRect.setAttributeNS(null, 'height', `${2 * circle.radius * config.cropScaleFactor}`);
  maskRect.setAttributeNS(null, 'fill', '#fff');
  const maskCircle = document.createElementNS(svgNamespace, 'circle');
  mask.appendChild(maskCircle);
  maskCircle.setAttributeNS(null, 'cx', `${circle.centerX}`);
  maskCircle.setAttributeNS(null, 'cy', `${circle.centerY}`);
  maskCircle.setAttributeNS(null, 'r', `${circle.radius}`);
  maskCircle.setAttributeNS(null, 'fill', '#000');
  const rect = document.createElementNS(svgNamespace, 'rect');
  svg.appendChild(rect);
  rect.setAttributeNS(null, 'x', `${circle.centerX - (circle.radius * config.cropScaleFactor)}`);
  rect.setAttributeNS(null, 'y', `${circle.centerY - (circle.radius * config.cropScaleFactor)}`);
  rect.setAttributeNS(null, 'width', `${2 * circle.radius * config.cropScaleFactor}`);
  rect.setAttributeNS(null, 'height', `${2 * circle.radius * config.cropScaleFactor}`);
  rect.setAttributeNS(null, 'fill', 'rgba(255, 255, 255, 0.5)');
  rect.setAttributeNS(null, 'mask', `url(#remove-radius-circle-${i})`);
};

const updateVisibleImages = entries => {
  entries.forEach(entry => {
    const i = parseInt(entry.target.id);
    if (entry.isIntersecting) {
      let circle = annotationCircles[i];

      const div = document.createElement('div');
      entry.target.appendChild(div);

      const svg = document.createElementNS(svgNamespace, 'svg');
      div.appendChild(svg);
      createTransparentCircle(i, svg, circle);

      let circleElement = null;
      let mouseDownX = null;
      let mouseDownY = null;
      let mouseDownTimestamp = null;
      let mouseDownShiftPressed = null;

      const transformEventToCircle = (x, y) => {
        return [
          (x / config.gridImageSize * 2 * circle.radius * config.cropScaleFactor) + circle.centerX - (circle.radius * config.cropScaleFactor),
          (y / config.gridImageSize * 2 * circle.radius * config.cropScaleFactor) + circle.centerY - (circle.radius * config.cropScaleFactor),
        ];
      };

      svg.onmousedown = event => {
        [mouseDownX, mouseDownY] = transformEventToCircle(event.offsetX, event.offsetY);
        mouseDownTimestamp = new Date;
        mouseDownShiftPressed = event.shiftKey;

        if (!mouseDownShiftPressed) {
          while (svg.firstChild) {
            svg.removeChild(svg.firstChild);
          }

          circleElement = document.createElementNS(svgNamespace, 'circle');
          svg.appendChild(circleElement);
          circleElement.setAttributeNS(null, 'cx', `${mouseDownX}`);
          circleElement.setAttributeNS(null, 'cy', `${mouseDownY}`);
          circleElement.setAttributeNS(null, 'r', '0');
          circleElement.setAttributeNS(null, 'fill', 'none');
          circleElement.setAttributeNS(null, 'stroke', '#fff');
          circleElement.setAttributeNS(null, 'stroke-width', `${2 * circle.radius * config.cropScaleFactor / config.gridImageSize}`);
        }

        return false;
      };

      svg.onmousemove = event => {
        if (circleElement !== null && mouseDownX !== null && mouseDownY !== null && mouseDownShiftPressed === false) {
          const [x, y] = transformEventToCircle(event.offsetX, event.offsetY);
          const dragCenterX = mouseDownX + ((x - mouseDownX) / 2);
          const dragCenterY = mouseDownY + ((y - mouseDownY) / 2);
          const dragRadius = Math.sqrt(((x - dragCenterX) * (x - dragCenterX)) + ((y - dragCenterY) * (y - dragCenterY)));

          circleElement.setAttributeNS(null, 'cx', `${dragCenterX}`);
          circleElement.setAttributeNS(null, 'cy', `${dragCenterY}`);
          circleElement.setAttributeNS(null, 'r', `${dragRadius}`);
        }

        return false;
      }

      svg.onmouseup = event => {
        let centerX = circle.centerX;
        let centerY = circle.centerY;
        let radius = circle.radius;

        if (!mouseDownShiftPressed) {
          const [x, y] = transformEventToCircle(event.offsetX, event.offsetY);

          const dragCenterX = mouseDownX + ((x - mouseDownX) / 2);
          const dragCenterY = mouseDownY + ((y - mouseDownY) / 2);
          const dragRadius = Math.sqrt(((x - dragCenterX) * (x - dragCenterX)) + ((y - dragCenterY) * (y - dragCenterY)));

          if (dragRadius > 2) { // only allow circles with minimal radius (too small circles cannot be removed easily)
            centerX = dragCenterX;
            centerY = dragCenterY;
            radius = dragRadius;
          } else {
            centerX = null;
            centerY = null;
            radius = null;
          }

          circleElement.setAttributeNS(null, 'stroke', 'none');
        } else if (event.shiftKey) {
          radius *= config.radiusIncreaseFactor;
        }

        while (svg.firstChild) {
          svg.removeChild(svg.firstChild);
        }

        if (centerX !== null && centerY !== null && radius !== null) {
          annotations[circle.image][circle.circleIndex].centerX = centerX;
          annotations[circle.image][circle.circleIndex].centerY = centerY;
          annotations[circle.image][circle.circleIndex].radius = radius;
          annotationCircles[i].centerX = centerX;
          annotationCircles[i].centerY = centerY;
          annotationCircles[i].radius = radius;
          circle = annotationCircles[i];

          (async () => {
            await fetch(`/set_circle/${i}`, {
              method: 'post',
              headers: {
                'Content-Type': 'application/json',
              },
              body: JSON.stringify(annotations[circle.image][circle.circleIndex]),
            });
            const image = new Image;
            image.onload = () => {
              img.src = image.currentSrc;
              createTransparentCircle(i, svg, circle);
            };
            image.src = `/image/${circle.imageIndex}?crop&centerX=${centerX}&centerY=${centerY}&radius=${radius * config.cropScaleFactor}&scale&width=${config.gridImageSize}&height=${config.gridImageSize}`;
          })();
          addTelemetryMessage({
            'timestamp': (new Date).toISOString(),
            'timestampDown': mouseDownTimestamp.toISOString(),
            'type': 'setCircle',
            'cellId': i,
            'image': circle.image,
          });
        } else {
          createTransparentCircle(i, svg, circle);
        }

        mouseDownX = null;
        mouseDownY = null;
        // mouseDownTimestamp is intentionally kept
        mouseDownShiftPressed = null;

        return false;
      };

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
};
