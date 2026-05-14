import base64
import io

import anywidget
import numpy
import traitlets
from PIL import Image


class MujocoViewer(anywidget.AnyWidget):
    _esm = """
    export default {
        render({ model, el }) {
            const canvas = document.createElement("canvas");
            const context = canvas.getContext("2d");

            canvas.style.borderRadius = "8px";
            canvas.style.boxShadow = "0 4px 6px rgba(0,0,0,0.1)";
            canvas.style.width = "100%";
            el.appendChild(canvas);

            // Measures the DOM element and sends the width back to the Python kernel
            const resize_observer = new ResizeObserver((entries) => {
                for (let entry of entries) {
                    const width = Math.floor(entry.contentRect.width);
                    if (width > 0 && width !== model.get("container_width")) {
                        model.set("container_width", width);
                        model.save_changes();
                    }
                }
            });
            resize_observer.observe(el);

            model.on("change:image_data", () => {
                const image_data = model.get("image_data");
                if (!image_data) return;

                const image = new Image();
                image.onload = () => {
                    if (canvas.width !== image.width) canvas.width = image.width;
                    if (canvas.height !== image.height) canvas.height = image.height;
                    context.drawImage(image, 0, 0);
                };
                image.src = "data:image/jpeg;base64," + image_data;
            });

            // Required to prevent memory leaks if the widget is destroyed
            return () => resize_observer.disconnect();
        }
    };
    """
    image_data = traitlets.Unicode("").tag(sync=True)
    container_width = traitlets.Int(640).tag(sync=True)

    def update(self, image_array: numpy.ndarray) -> None:
        image = Image.fromarray(image_array)
        buffer = io.BytesIO()
        image.save(buffer, format="JPEG")
        self.image_data = base64.b64encode(buffer.getvalue()).decode("utf-8")
