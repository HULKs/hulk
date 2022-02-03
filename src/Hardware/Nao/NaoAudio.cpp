#include "Hardware/Nao/NaoAudio.hpp"
#include "Framework/Log/Log.hpp"

NaoAudio::NaoAudio()
  : captureSampleRate_(captureSamplingRate)
  , playbackSampleRate_(playbackSamplingRate)
  , runCaptureThread_(true)
  , runPlaybackThread_(true)
{
  initCapture();
  initPlayback();

  properties_.playbackSupported = true;
  properties_.recordingSupported = true;
}

void NaoAudio::initCapture()
{
  int err;
  snd_pcm_hw_params_t* hw_params;

  // device name set in .asoundrc
  if ((err = snd_pcm_open(&captureHandle_, "default", SND_PCM_STREAM_CAPTURE, 0)) < 0)
  {
    fprintf(stderr, "cannot open audio device %s (%s)\n", "PCH_input", snd_strerror(err));
    exit(1);
  }

  if ((err = snd_pcm_hw_params_malloc(&hw_params)) < 0)
  {
    fprintf(stderr, "cannot allocate hardware parameter structure (%s)\n", snd_strerror(err));
    exit(1);
  }

  // choose all capture parameters
  if ((err = snd_pcm_hw_params_any(captureHandle_, hw_params)) < 0)
  {
    fprintf(stderr, "cannot initialize hardware parameter structure (%s)\n", snd_strerror(err));
    exit(1);
  }

  // set the interleaved read format
  if ((err = snd_pcm_hw_params_set_access(captureHandle_, hw_params,
                                          SND_PCM_ACCESS_RW_INTERLEAVED)) < 0)
  {
    fprintf(stderr, "cannot set access type (%s)\n", snd_strerror(err));
    exit(1);
  }

  // set the capture sample format
  if ((err = snd_pcm_hw_params_set_format(captureHandle_, hw_params, SND_PCM_FORMAT_FLOAT)) < 0)
  {
    fprintf(stderr, "cannot set sample format (%s)\n", snd_strerror(err));
    exit(1);
  }

  // set the capture stream rate
  if ((err = snd_pcm_hw_params_set_rate_near(captureHandle_, hw_params, &captureSampleRate_, 0)) <
      0)
  {
    fprintf(stderr, "cannot set sample rate (%s)\n", snd_strerror(err));
    exit(1);
  }
  if (captureSampleRate_ != captureSamplingRate)
  {
    fprintf(stderr, "Capture rate doesn't match (requested %uHz, get %iHz)\n", captureSamplingRate,
            captureSampleRate_);
  }

  // set the count of microphone channels
  if ((err = snd_pcm_hw_params_set_channels(captureHandle_, hw_params, numChannels)) < 0)
  {
    fprintf(stderr, "cannot set channel count (%s)\n", snd_strerror(err));
    exit(1);
  }

  // write the parameters to device
  if ((err = snd_pcm_hw_params(captureHandle_, hw_params)) < 0)
  {
    fprintf(stderr, "cannot set parameters (%s)\n", snd_strerror(err));
    exit(1);
  }

  snd_pcm_hw_params_free(hw_params);
}

void NaoAudio::initPlayback()
{
  int err;
  snd_pcm_hw_params_t* hw_params;

  if ((err = snd_pcm_open(&playbackHandle_, "default", SND_PCM_STREAM_PLAYBACK, 0)) < 0)
  {
    fprintf(stderr, "cannot open audio device %s (%s)\n", "default", snd_strerror(err));
    exit(1);
  }

  if ((err = snd_pcm_hw_params_malloc(&hw_params)) < 0)
  {
    fprintf(stderr, "cannot allocate hardware parameter structure (%s)\n", snd_strerror(err));
    exit(1);
  }

  // choose all playback parameters
  if ((err = snd_pcm_hw_params_any(playbackHandle_, hw_params)) < 0)
  {
    fprintf(stderr, "cannot initialize hardware parameter structure (%s)\n", snd_strerror(err));
    exit(1);
  }

  // set the interleaved write format
  if ((err = snd_pcm_hw_params_set_access(playbackHandle_, hw_params,
                                          SND_PCM_ACCESS_RW_INTERLEAVED)) < 0)
  {
    fprintf(stderr, "cannot set access type (%s)\n", snd_strerror(err));
    exit(1);
  }

  // set the playback sample format
  if ((err = snd_pcm_hw_params_set_format(playbackHandle_, hw_params, SND_PCM_FORMAT_FLOAT)) < 0)
  {
    fprintf(stderr, "cannot set sample format (%s)\n", snd_strerror(err));
    exit(1);
  }

  // set the playback stream rate
  if ((err = snd_pcm_hw_params_set_rate_near(playbackHandle_, hw_params, &playbackSampleRate_, 0)) <
      0)
  {
    fprintf(stderr, "cannot set sample rate (%s)\n", snd_strerror(err));
    exit(1);
  }
  if (playbackSampleRate_ != playbackSamplingRate)
  {
    fprintf(stderr, "Rate doesn't match (requested %uHz, get %iHz)\n", playbackSamplingRate,
            playbackSampleRate_);
    exit(1);
  }

  // set the count of playback channels
  // Note: Output is set to mono, because stereo isn't needed right now.
  if ((err = snd_pcm_hw_params_set_channels(playbackHandle_, hw_params, 1)) < 0)
  {
    fprintf(stderr, "cannot set channel count (%s)\n", snd_strerror(err));
    exit(1);
  }

  // write the parameters to device
  if ((err = snd_pcm_hw_params(playbackHandle_, hw_params)) < 0)
  {
    fprintf(stderr, "cannot set parameters (%s)\n", snd_strerror(err));
    exit(1);
  }

  snd_pcm_hw_params_free(hw_params);
}

void NaoAudio::startCapture()
{
  int err;
  if ((err = snd_pcm_prepare(captureHandle_)) < 0)
  {
    fprintf(stderr, "cannot prepare audio interface for use (%s)\n", snd_strerror(err));
    exit(1);
  }

  captureThread_ = std::thread([this]() {
    int err;
    float buf[numChannels * framesPerBuffer];
    while (runCaptureThread_)
    {
      if ((err = snd_pcm_readi(captureHandle_, buf, framesPerBuffer)) != framesPerBuffer)
      {
        fprintf(stderr, "read from audio interface failed (%s)\n", snd_strerror(err));
        exit(1);
      }
      // write into buffer, that will be read in audioReceiver
      std::lock_guard<std::mutex> lg(inBufferLock_);
      for (unsigned int micId = 0; micId < numChannels; micId++)
      {
        for (unsigned int i = micId; i < framesPerBuffer * numChannels; i += numChannels)
        {
          inBuffer_[micId].buffer.push_back(buf[i]);
        }
      }
    }
  });
}

void NaoAudio::stopCapture()
{
  runCaptureThread_ = false;
  captureThread_.join();
}

void NaoAudio::startPlayback()
{
  int err;

  if ((err = snd_pcm_prepare(playbackHandle_)) < 0)
  {
    fprintf(stderr, "cannot prepare audio interface for use (%s)\n", snd_strerror(err));
    exit(1);
  }

  playbackThread_ = std::thread([this]() {
    snd_pcm_sframes_t frames;

    while (runPlaybackThread_)
    {
      std::unique_lock<std::mutex> lg(outBuffer_.lock);
      // wait until there is data to playback in the buffer or until the thread gets closed
      playbackCondition_.wait(
          lg, [this] { return !runPlaybackThread_ || outBuffer_.buffer.size() > 0; });

      float buf[framesPerBuffer];
      std::memset(buf, 0, sizeof(buf));
      int bufSize = std::min((int)framesPerBuffer, (int)outBuffer_.buffer.size());
      if (outBuffer_.buffer.size() > 0)
      {
        for (int i = 0; i < bufSize; i++)
        {
          buf[i] = outBuffer_.buffer.front();
          outBuffer_.buffer.pop_front();
        }
      }
      else
      {
        continue;
      }

      if ((frames = snd_pcm_writei(playbackHandle_, &buf, bufSize)) != bufSize)
      {
        frames = snd_pcm_recover(playbackHandle_, frames, 0);
        if (frames < 0)
        {
          fprintf(stderr, "write to audio interface failed (%s)\n", snd_strerror(frames));
          exit(1);
        }
        if (frames > 0 && frames < (long)sizeof(buf))
        {
          printf("Short write (expected %li, wrote %li)\n", (long)sizeof(buf), frames);
        }
      }
    }
  });
}

void NaoAudio::stopPlayback()
{
  runPlaybackThread_ = false;
  playbackCondition_.notify_all();
  playbackThread_.join();
}

bool NaoAudio::isPlaybackFinished()
{
  return outBuffer_.buffer.empty();
}

void NaoAudio::clearPlaybackBuffer()
{
  outBuffer_.buffer.clear();
}

AudioInterface::AudioProperties NaoAudio::getAudioProperties()
{
  return properties_;
}

void NaoAudio::readAudioData(std::array<SampleRingBuffer, numChannels>& recordData,
                             std::array<SampleRingBufferIt, numChannels>& cycleStartIterators)
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

void NaoAudio::playbackAudioData(const Samples& samples)
{
  assert(properties_.playbackSupported);
  std::lock_guard<std::mutex> lg(outBuffer_.lock);
  outBuffer_.buffer.insert(outBuffer_.buffer.end(), samples.begin(), samples.end());
  playbackCondition_.notify_all();
}

NaoAudio::~NaoAudio()
{
  snd_pcm_close(captureHandle_);
  snd_pcm_close(playbackHandle_);
}
