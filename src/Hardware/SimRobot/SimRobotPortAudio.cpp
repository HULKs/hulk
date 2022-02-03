#include <thread>

#include "Framework/Log/Log.hpp"
#include "Hardware/SimRobot/SimRobotPortAudio.hpp"

SimRobotPortAudio::SimRobotPortAudio()
{
  PaError err;
  // Input
  PaStreamParameters inputParameters;

  // Just after booting it seems that the audio devices are not available already.
  // Therefore it has to be retried some times.
  // It turned out that it is also important to call Pa_Initialize each try.
  for (std::uint8_t time = 0; time < 10; time++)
  {
    err = Pa_Initialize();
    if (err != paNoError)
    {
      Log<M_TUHHSDK>(LogLevel::ERROR) << "PortAudio generated an error: " << err;
      return;
    }

    inputParameters.device = Pa_GetDefaultInputDevice();
    if (inputParameters.device != paNoDevice)
    {
      break;
    }

    Pa_Terminate();
    Log<M_TUHHSDK>(LogLevel::INFO) << "Could not open PortAudio input device, will retry.";
    std::this_thread::sleep_for(std::chrono::seconds(1));
  }

  if (inputParameters.device == paNoDevice)
  {
    throw std::runtime_error("No Default input device found.");
  }

  inputParameters.channelCount = numChannels;
  inputParameters.sampleFormat = paFloat32; // 32 bit floating point output
  inputParameters.suggestedLatency =
      Pa_GetDeviceInfo(inputParameters.device)->defaultLowInputLatency;
  inputParameters.hostApiSpecificStreamInfo = NULL;

  err = Pa_OpenStream(&inStream_, &inputParameters, NULL, captureSamplingRate, framesPerBuffer,
                      paClipOff, SimRobotPortAudio::recordCallback, this);
  handlePaErrorCode(err);

  err = Pa_SetStreamFinishedCallback(inStream_, SimRobotPortAudio::recordFinishedCallback);
  handlePaErrorCode(err);

  // Output
  PaStreamParameters outputParameters;
  outputParameters.device = Pa_GetDefaultOutputDevice();
  if (outputParameters.device == paNoDevice)
  {
    throw std::runtime_error("No Default output device found.");
  }

  outputParameters.channelCount = 2;         // stereo output
  outputParameters.sampleFormat = paFloat32; // 32 bit floating point output
  outputParameters.suggestedLatency =
      Pa_GetDeviceInfo(outputParameters.device)->defaultLowOutputLatency;
  outputParameters.hostApiSpecificStreamInfo = NULL;

  err = Pa_OpenStream(&outStream_, NULL, &outputParameters, playbackSamplingRate, framesPerBuffer,
                      paClipOff, SimRobotPortAudio::playbackCallback, this);
  handlePaErrorCode(err);

  err = Pa_SetStreamFinishedCallback(outStream_, SimRobotPortAudio::playbackFinishedCallback);
  handlePaErrorCode(err);

  properties_.playbackSupported = true;
  properties_.recordingSupported = true;
}

void SimRobotPortAudio::startCapture()
{
  PaError err = Pa_StartStream(inStream_);
  handlePaErrorCode(err);
}

void SimRobotPortAudio::stopCapture()
{
  PaError err = Pa_StopStream(inStream_);
  handlePaErrorCode(err);
}

void SimRobotPortAudio::startPlayback()
{
  PaError err = Pa_StartStream(outStream_);
  handlePaErrorCode(err);
}

void SimRobotPortAudio::stopPlayback()
{
  PaError err = Pa_StopStream(outStream_);
  handlePaErrorCode(err);
}

int SimRobotPortAudio::playbackCallback(const void* /*inputBuffer*/, void* outputBuffer,
                                        unsigned long framesPerBuffer,
                                        const PaStreamCallbackTimeInfo* /*timeInfo*/,
                                        PaStreamCallbackFlags /*statusFlags*/, void* userData)
{
  SimRobotPortAudio* self = (SimRobotPortAudio*)userData;
  float* out = (float*)outputBuffer;

  std::lock_guard<std::mutex> lg(self->outBuffer_.lock);
  for (unsigned int i = 0; i < framesPerBuffer; ++i)
  {
    float sample = 0.0f;
    if (self->outBuffer_.buffer.size())
    {
      sample = self->outBuffer_.buffer.front();
      self->outBuffer_.buffer.pop_front();
    }

    *out++ = sample;
    *out++ = sample;
  }

  return paContinue;
}

bool SimRobotPortAudio::isPlaybackFinished()
{
  return outBuffer_.buffer.empty();
}

void SimRobotPortAudio::clearPlaybackBuffer()
{
  outBuffer_.buffer.clear();
}

int SimRobotPortAudio::recordCallback(const void* inputBuffer, void* /*outputBuffer*/,
                                      unsigned long framesPerBuffer,
                                      const PaStreamCallbackTimeInfo* /*timeInfo*/,
                                      PaStreamCallbackFlags /*statusFlags*/, void* userData)
{
  SimRobotPortAudio* self = (SimRobotPortAudio*)userData;
  float* in = (float*)inputBuffer;

  // inputBuffer contains the data of all micros interlayed.
  // e.g. [rearLeft0, rearRight0, frontLeft0, frontRight0
  //       rearLeft1...]
  for (unsigned int micId = 0; micId < numChannels; micId++)
  {
    std::lock_guard<std::mutex> lg(self->inBuffer_[micId].lock);
    for (unsigned int i = micId; i < framesPerBuffer * numChannels; i += numChannels)
    {
      self->inBuffer_[micId].buffer.push_back(in[i]);
    }
  }

  return paContinue;
}

void SimRobotPortAudio::playbackFinishedCallback(void* /*userData*/)
{
  Log<M_TUHHSDK>(LogLevel::DEBUG) << "Playback finished";
}

void SimRobotPortAudio::recordFinishedCallback(void* /*userData*/)
{
  Log<M_TUHHSDK>(LogLevel::DEBUG) << "Capture finished";
}

SimRobotPortAudio::~SimRobotPortAudio()
{
  PaError err = Pa_CloseStream(outStream_);
  handlePaErrorCode(err);

  err = Pa_CloseStream(inStream_);
  handlePaErrorCode(err);

  err = Pa_Terminate();
  handlePaErrorCode(err);
}

AudioInterface::AudioProperties SimRobotPortAudio::getAudioProperties()
{
  return properties_;
}

void SimRobotPortAudio::readAudioData(
    std::array<SampleRingBuffer, AudioInterface::numChannels>& recordData,
    std::array<SampleRingBufferIt, AudioInterface::numChannels>& cycleStartIterators)
{
  assert(properties_.recordingSupported);
  std::lock_guard<std::mutex> lg(inBufferLock_);
  for (unsigned int channel = 0; channel < numChannels; channel++)
  {
    cycleStartIterators[channel] = recordData[channel].end() - 1;
    recordData[channel].insert(recordData[channel].end(), inBuffer_[channel].buffer.begin(),
                               inBuffer_[channel].buffer.end());
    inBuffer_[channel].buffer.clear();
  }
}

void SimRobotPortAudio::playbackAudioData(const Samples& samples)
{
  assert(properties_.playbackSupported);
  std::lock_guard<std::mutex> lg(outBuffer_.lock);
  outBuffer_.buffer.insert(outBuffer_.buffer.end(), samples.begin(), samples.end());
}

void SimRobotPortAudio::handlePaErrorCode(int err)
{
  if (err != paNoError)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "PortAudio generated an Error: " << err;
  }
}
