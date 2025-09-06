import av
import numpy as np


class H264Encoder:
    def __init__(
        self, *, width: int, height: int, fps: int = 30, lossless: bool = True
    ) -> None:
        self.width = width
        self.height = height
        self.fps = fps

        # Create an in-memory output container
        self.container = av.open("dummy.mp4", mode="w", format="mp4")
        self.stream = self.container.add_stream("libx264", rate=fps)

        # Set options for lossless
        if lossless:
            self.stream.options = {"qp": "0", "preset": "ultrafast"}
        else:
            self.stream.options = {"preset": "ultrafast"}

    def encode_frame(self, frame: np.ndarray) -> bytes:
        """
        Encode a single RGB frame (numpy array) and return H.264 bytes.
        """
        # Convert numpy array to VideoFrame
        video_frame = av.VideoFrame.from_ndarray(frame, format="rgb24")

        encoded_bytes = b""
        for packet in self.stream.encode(video_frame):
            encoded_bytes += bytes(packet)

        # print(self.stream.codec_context.extradata) # contains sps and pps (annex-b)
        return encoded_bytes

    def flush(self) -> bytes:
        """
        Flush remaining packets in the encoder.
        """
        encoded_bytes = b""
        for packet in self.stream.encode():
            encoded_bytes += bytes(packet)
        return encoded_bytes
