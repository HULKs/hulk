let maximizedCellElement = null;

let minimizedBoxX = null;
let minimizedBoxY = null;
let minimizedBoxWidth = null;
let minimizedBoxHeight = null;
let maximizeAnimation = null;

let mouseDownX = null;
let mouseDownY = null;
let mouseDownInCircle = false;
let mouseDownOutside = false;
let mouseDownTimestamp = null;

const map = (value, sourceStart, sourceEnd, targetStart, targetEnd) => {
  return ((value - sourceStart) / (sourceEnd - sourceStart) * (targetEnd - targetStart)) + targetStart;
};

const updateCircles = () => {
  const i = parseInt(maximizedCellElement.id);
  const existingCircles = existingAnnotations[annotationImages[i]];
  const addedCircles = addedAnnotations[annotationImages[i]];
  const groupElement = document.querySelector('#box g');

  while (groupElement.firstChild) {
    groupElement.removeChild(groupElement.firstChild);
  }

  addedCircles.forEach(circle => {
    const rect = document.createElementNS(svgNamespace, 'rect');
    groupElement.appendChild(rect);
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
};

const maximize = cellElement => {
  maximizedCellElement = cellElement;
  const i = parseInt(maximizedCellElement.id);
  addTelemetryMessage({
    'timestamp': (new Date).toISOString(),
    'type': 'maximize',
    'cellId': i,
    'image': annotationImages[i],
  });
  const svg = maximizedCellElement.querySelector('div>svg');
  const boundingBox = svg.getBoundingClientRect();
  minimizedBoxX = window.scrollX + boundingBox.left;
  minimizedBoxY = window.scrollY + boundingBox.top;
  minimizedBoxWidth = boundingBox.width;
  minimizedBoxHeight = boundingBox.height;
  const maximizedBoxX = window.scrollX;
  const maximizedBoxY = window.scrollY;
  const maximizedBoxWidth = window.innerWidth;
  const maximizedBoxHeight = window.innerHeight;

  const boxElement = document.querySelector('#box');
  const image = new Image;
  image.onload = () => {
    boxElement.setAttribute('viewBox', `0 0 ${image.naturalWidth} ${image.naturalHeight}`);
    const imageElement = boxElement.querySelector('image');
    imageElement.setAttributeNS(null, 'href', image.currentSrc);
    imageElement.setAttributeNS(null, 'x', '0');
    imageElement.setAttributeNS(null, 'y', '0');
    imageElement.setAttributeNS(null, 'width', `${image.naturalWidth}`);
    imageElement.setAttributeNS(null, 'height', `${image.naturalHeight}`);

    updateCircles();

    const timing = {
      duration: 300,
      easing: 'cubic-bezier(0.4, 0.0, 0.2, 1)',
      fill: 'forwards',
    };
    if (maximizeAnimation !== null) {
      maximizeAnimation.pause();
    }
    maximizeAnimation = boxElement.animate([
      {
        left: `${minimizedBoxX}px`,
        top: `${minimizedBoxY}px`,
        width: `${minimizedBoxWidth}px`,
        height: `${minimizedBoxHeight}px`,
      },
      {
        left: `${maximizedBoxX}px`,
        top: `${maximizedBoxY}px`,
        width: `${maximizedBoxWidth}px`,
        height: `${maximizedBoxHeight}px`,
      },
    ], timing);
    maximizeAnimation.onfinish = () => {
      maximizeAnimation = null;
    };

    boxElement.style.visibility = 'visible';
  };
  image.src = `/image/${i}`;
};

const minimize = callback => {
  if (minimizedBoxX === null || minimizedBoxY === null || minimizedBoxWidth === null || minimizedBoxHeight === null) {
    return;
  }

  const boxElement = document.querySelector('#box');
  const currentStyleBox = window.getComputedStyle(boxElement);
  const i = parseInt(maximizedCellElement.id);
  addTelemetryMessage({
    'timestamp': (new Date).toISOString(),
    'type': 'minimize',
    'cellId': i,
    'image': annotationImages[i],
  });

  const timing = {
    duration: 200,
    easing: 'cubic-bezier(0.4, 0.0, 0.2, 1)',
    fill: 'forwards',
  };
  if (maximizeAnimation !== null) {
    maximizeAnimation.pause();
  }
  maximizeAnimation = boxElement.animate([
    {
      left: currentStyleBox.left,
      top: currentStyleBox.top,
      width: currentStyleBox.width,
      height: currentStyleBox.height,
    },
    {
      left: `${minimizedBoxX}px`,
      top: `${minimizedBoxY}px`,
      width: `${minimizedBoxWidth}px`,
      height: `${minimizedBoxHeight}px`,
    },
  ], timing);
  maximizeAnimation.onfinish = () => {
    maximizeAnimation = null;
    boxElement.style.visibility = 'hidden';
    if (callback) {
      callback();
    }
  };
};

const setupMaximize = () => {
  const boxElement = document.querySelector('#box');
  const circleElement = boxElement.querySelector('circle');

  const transformViewportToImage = (x, y) => {
    const viewportBoundingBox = boxElement.getBoundingClientRect();
    const viewportRatio = viewportBoundingBox.width / viewportBoundingBox.height;
    const imageViewBoxParts = boxElement.getAttributeNS(null, 'viewBox').split(' ');
    const imageBoundingBox = { width: parseInt(imageViewBoxParts[2]), height: parseInt(imageViewBoxParts[3]) };
    const imageRatio = imageBoundingBox.width / imageBoundingBox.height;
    const imageWidthInViewport = viewportRatio < imageRatio ? viewportBoundingBox.width : imageBoundingBox.width / imageBoundingBox.height * viewportBoundingBox.height;
    const imageHeightInViewport = viewportRatio < imageRatio ? imageBoundingBox.height / imageBoundingBox.width * viewportBoundingBox.width : viewportBoundingBox.height;
    const imageOffsetXInViewport = (viewportBoundingBox.width - imageWidthInViewport) / 2;
    const imageOffsetYInViewport = (viewportBoundingBox.height - imageHeightInViewport) / 2;

    return [
      (x - imageOffsetXInViewport) / imageWidthInViewport * imageBoundingBox.width,
      (y - imageOffsetYInViewport) / imageHeightInViewport * imageBoundingBox.height,
    ];
  };

  boxElement.onmousedown = event => {
    [mouseDownX, mouseDownY] = transformViewportToImage(event.x, event.y);
    mouseDownTimestamp = new Date;

    const i = parseInt(maximizedCellElement.id);
    const image = annotationImages[i];
    const addedCircles = addedAnnotations[image];
    mouseDownInCircle = addedCircles.some(circle =>
      mouseDownX >= circle.centerX - circle.radius && mouseDownX <= circle.centerX + circle.radius &&
      mouseDownY >= circle.centerY - circle.radius && mouseDownY <= circle.centerY + circle.radius
    );
    const imageViewBoxParts = boxElement.getAttributeNS(null, 'viewBox').split(' ');
    mouseDownOutside = mouseDownX < 0 || mouseDownX > parseInt(imageViewBoxParts[2]) || mouseDownY < 0 || mouseDownY > parseInt(imageViewBoxParts[3]);

    if (!mouseDownInCircle && !mouseDownOutside) {
      circleElement.setAttributeNS(null, 'cx', `${mouseDownX}`);
      circleElement.setAttributeNS(null, 'cy', `${mouseDownY}`);
      circleElement.setAttributeNS(null, 'r', '0');
      circleElement.setAttributeNS(null, 'stroke', `rgb(${config.addedAnnotationColor[0]}, ${config.addedAnnotationColor[1]}, ${config.addedAnnotationColor[2]})`);
    }

    return false;
  };

  boxElement.onmousemove = event => {
    if (!mouseDownInCircle && !mouseDownOutside && mouseDownX !== null && mouseDownY !== null) {
      const [x, y] = transformViewportToImage(event.x, event.y);
      const dragCenterX = mouseDownX + ((x - mouseDownX) / 2);
      const dragCenterY = mouseDownY + ((y - mouseDownY) / 2);
      const dragRadius = Math.sqrt(((x - dragCenterX) * (x - dragCenterX)) + ((y - dragCenterY) * (y - dragCenterY)));

      circleElement.setAttributeNS(null, 'cx', `${dragCenterX}`);
      circleElement.setAttributeNS(null, 'cy', `${dragCenterY}`);
      circleElement.setAttributeNS(null, 'r', `${dragRadius}`);
    }
    return false;
  }

  boxElement.onmouseup = event => {
    const [x, y] = transformViewportToImage(event.x, event.y);
    const i = parseInt(maximizedCellElement.id);

    if (mouseDownInCircle) {
      const image = annotationImages[i];
      const addedCircles = addedAnnotations[image];
      const existingCircles = existingAnnotations[image];

      const isInsideCircle = (x, y, centerX, centerY, radius) =>
        x >= centerX - radius && x <= centerX + radius &&
        y >= centerY - radius && y <= centerY + radius;
      const isExistingCircle = (centerX, centerY, radius) =>
        existingCircles.some(existingCircle => existingCircle.centerX === centerX && existingCircle.centerY === centerY && existingCircle.radius === radius);

      addedAnnotations[image] = addedCircles.filter(circle =>
        isExistingCircle(circle.centerX, circle.centerY, circle.radius) ||
        !isInsideCircle(mouseDownX, mouseDownY, circle.centerX, circle.centerY, circle.radius) ||
        !isInsideCircle(x, y, circle.centerX, circle.centerY, circle.radius));
      updateCircles();
      (async () => {
        await fetch(`/set_added/${i}`, {
          method: 'post',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(addedAnnotations[image]),
        });
      })();
      addTelemetryMessage({
        'timestamp': (new Date).toISOString(),
        'timestampDown': mouseDownTimestamp.toISOString(),
        'type': 'removeCircle',
      });
    } if (mouseDownOutside) {
      const imageViewBoxParts = boxElement.getAttributeNS(null, 'viewBox').split(' ');
      if (x < 0 || x > parseInt(imageViewBoxParts[2]) || y < 0 || y > parseInt(imageViewBoxParts[3])) {
        // trigger minimize
        history.back();
      }
    } else {
      const dragCenterX = mouseDownX + ((x - mouseDownX) / 2);
      const dragCenterY = mouseDownY + ((y - mouseDownY) / 2);
      const dragRadius = Math.sqrt(((x - dragCenterX) * (x - dragCenterX)) + ((y - dragCenterY) * (y - dragCenterY)));

      circleElement.setAttributeNS(null, 'stroke', 'none');

      if (dragRadius > 2) { // only allow circles with minimal radius (too small circles cannot be removed easily)
        addedAnnotations[annotationImages[i]].push({
          centerX: dragCenterX,
          centerY: dragCenterY,
          radius: dragRadius,
        });
        updateCircles();
        (async () => {
          await fetch(`/set_added/${i}`, {
            method: 'post',
            headers: {
              'Content-Type': 'application/json',
            },
            body: JSON.stringify(addedAnnotations[annotationImages[i]]),
          });
        })();
        addTelemetryMessage({
          'timestamp': (new Date).toISOString(),
          'timestampDown': mouseDownTimestamp.toISOString(),
          'type': 'addCircle',
        });
      }
    }
    mouseDownX = null;
    mouseDownY = null;
    return false;
  };
};
