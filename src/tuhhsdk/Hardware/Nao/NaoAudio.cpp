#include <thread>

#include "NaoAudio.hpp"
#include "print.h"

NaoAudio::NaoAudio()
{
  PaError err;
  //Input
  PaStreamParameters inputParameters;

  // Just after booting it seems that the audio devices are not available already.
  // Therefore it has to be retried some times.
  // It turned out that it is also important to call Pa_Initialize each try.
  for (std::uint8_t time = 0; time < 10; time++)
  {
    err = Pa_Initialize();
    if (err != paNoError)
    {
      print("PortAudio generated an error: ", err, LogLevel::ERROR);
      return;
    }

    inputParameters.device = Pa_GetDefaultInputDevice();
    if (inputParameters.device != paNoDevice)
    {
      break;
    }

    Pa_Terminate();
    print("Could not open PortAudio input device, will retry.", LogLevel::INFO);
    std::this_thread::sleep_for(std::chrono::seconds(1));
  }

  if (inputParameters.device == paNoDevice) {
    throw std::runtime_error("No Default input device found.");
  }

  inputParameters.channelCount = 1;
  inputParameters.sampleFormat = paFloat32; // 32 bit floating point output
  inputParameters.suggestedLatency = Pa_GetDeviceInfo(inputParameters.device)->defaultLowInputLatency;
  inputParameters.hostApiSpecificStreamInfo = NULL;

  err = Pa_OpenStream(&inStream_, &inputParameters, NULL, samplingRate, framesPerBuffer, paClipOff, NaoAudio::recordCallback, this);
  handlePaErrorCode(err);

  err = Pa_SetStreamFinishedCallback(inStream_, NaoAudio::recordFinishedCallback);
  handlePaErrorCode(err);

  //Output
  PaStreamParameters outputParameters;
  outputParameters.device = Pa_GetDefaultOutputDevice();
  if (outputParameters.device == paNoDevice) {
    throw std::runtime_error("No Default output device found.");
  }

  outputParameters.channelCount = 2; // stereo output
  outputParameters.sampleFormat = paFloat32; // 32 bit floating point output
  outputParameters.suggestedLatency = Pa_GetDeviceInfo(outputParameters.device)->defaultLowOutputLatency;
  outputParameters.hostApiSpecificStreamInfo = NULL;

  err = Pa_OpenStream(&outStream_, NULL, &outputParameters, samplingRate, framesPerBuffer, paClipOff, NaoAudio::playbackCallback, this);
  handlePaErrorCode(err);

  err = Pa_SetStreamFinishedCallback(outStream_, NaoAudio::playbackFinishedCallback);
  handlePaErrorCode(err);
}

void NaoAudio::startCapture()
{
  PaError err = Pa_StartStream(inStream_);
  handlePaErrorCode(err);
}

void NaoAudio::stopCapture()
{
  PaError err = Pa_StopStream(inStream_);
  handlePaErrorCode(err);
}

void NaoAudio::startPlayback()
{
  PaError err = Pa_StartStream(outStream_);
  handlePaErrorCode(err);
}

void NaoAudio::stopPlayback()
{
  PaError err = Pa_StopStream(outStream_);
  handlePaErrorCode(err);
}

int NaoAudio::playbackCallback(const void* /*inputBuffer*/, void* outputBuffer,
                        unsigned long framesPerBuffer,
                        const PaStreamCallbackTimeInfo* /*timeInfo*/,
                        PaStreamCallbackFlags /*statusFlags*/,
                        void *userData)
{
  NaoAudio* self = (NaoAudio*)userData;
  float* out = (float*)outputBuffer;

  std::lock_guard<std::mutex> lg(self->outBuffer_.lock);
  for (unsigned int i = 0; i < framesPerBuffer; ++i) {
    float sample = 0.0f;
    if (self->outBuffer_.buffer.size()) {
      sample = self->outBuffer_.buffer.front();
      self->outBuffer_.buffer.pop_front();
    }

    *out++ = sample;
    *out++ = sample;
  }

  return paContinue;
}

int NaoAudio::recordCallback(const void* inputBuffer, void* /*outputBuffer*/,
                     unsigned long framesPerBuffer,
                     const PaStreamCallbackTimeInfo* /*timeInfo*/,
                     PaStreamCallbackFlags /*statusFlags*/,
                     void* userData)
{
  NaoAudio* self = (NaoAudio*)userData;
  float* in = (float*)inputBuffer;

  std::lock_guard<std::mutex> lg(self->inBuffer_.lock);
  for (unsigned int i = 0; i < framesPerBuffer; i++) {
    self->inBuffer_.buffer.push_back(in[i]);
  }

  return paContinue;
}

void NaoAudio::playbackFinishedCallback(void* /*userData*/)
{
  print("Playback finished!", LogLevel::DEBUG);
}

void NaoAudio::recordFinishedCallback(void* /*userData*/)
{
  print("Capture finished!", LogLevel::DEBUG);
}

NaoAudio::~NaoAudio()
{
  PaError err = Pa_CloseStream(outStream_);
  handlePaErrorCode(err);

  err = Pa_CloseStream(inStream_);
  handlePaErrorCode(err);

  err = Pa_Terminate();
  handlePaErrorCode(err);
}

void NaoAudio::readAudioData(Samples& audio_data)
{
  audio_data = Samples();
  std::lock_guard<std::mutex> lg(inBuffer_.lock);
  audio_data.insert(audio_data.end(), inBuffer_.buffer.begin(), inBuffer_.buffer.end());
  inBuffer_.buffer.clear();
}

void NaoAudio::playbackAudioData(const Samples& samples)
{
  std::lock_guard<std::mutex> lg(outBuffer_.lock);
  outBuffer_.buffer.insert(outBuffer_.buffer.end(), samples.begin(), samples.end());
}

void NaoAudio::handlePaErrorCode(int err)
{
  if (err != paNoError) {
    Log(LogLevel::ERROR) << "PortAudio generated an Error: " << err;
  }
}
