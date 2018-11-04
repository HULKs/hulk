#include <cmath>
#include <fstream>
#include <iomanip>
#include <print.hpp>

#include "BallDetectionNeuralNet.hpp"
#include "Modules/Debug/Debug.h"
#include "Modules/Debug/PngConverter.h"
#include "Tools/Chronometer.hpp"

#include "BallDetectionNeuralNet.hpp"

BallDetectionNeuralNet::BallDetectionNeuralNet(const ModuleManagerInterface& manager)
  : Module(manager)

  , seedRadiusRatioMin_(*this, "seedRadiusRatioMin", [] {})
  , seedRadiusRatioMax_(*this, "seedRadiusRatioMax", [] {})
  , seedDark_(*this, "seedDark", [] {})
  , seedBrightMin_(*this, "seedBrightMin", [] {})
  , seedBright_(*this, "seedBright", [] {})
  , seedBrightScore_(*this, "seedBrightScore", [] {})
  , candidateMinSeeds_(*this, "candidateMinSeeds", [] {})
  , projectFoundBalls_(*this, "projectFoundBalls", [] {})

  , netAccuracy_(*this, "netAccuracy", [] {})

  , imageData_(*this)
  , cameraMatrix_(*this)
  , imageSegments_(*this)
  , fieldBorder_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , cycleInfo_(*this)
  , ballState_(*this)
  , ballData_(*this)
  , candidateCount_(0)
  , writeCandidatesToDisk_(*this, "writeCandidatesToDisk", [] {})
{

  const std::string mount = "Brain.BallDetectionNeuralNet.Weights";
  configuration().mount(mount, "BallDetectionNeuralNet.Weights.json", ConfigurationType::HEAD);
  configuration().get(mount, "CW") >> netConvMask_;
  configuration().get(mount, "Cb") >> netConvBias_;
  configuration().get(mount, "netConvPooling") >> netConvPooling_;
  configuration().get(mount, "netConvActivation") >> netConvActivation_;
  configuration().get(mount, "N") >> netNorm_;
  configuration().get(mount, "FCW") >> netFCWeights_;
  configuration().get(mount, "FCb") >> netFCBias_;
  configuration().get(mount, "netFCActivation") >> netFCActivation_;
  configuration().get(mount, "netSampleSize") >> netSampleSize_;
}

void BallDetectionNeuralNet::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time." + imageData_->identification);

    const std::vector<Circle<int>> seeds = getSeeds();
    const std::vector<Circle<int>> candidates = mergeSeeds(seeds);
    std::vector<Circle<int>> balls;
    if (applyFilter(candidates, balls))
    {
      for (auto& circle : balls)
      {
        Vector2f pos;
        cameraMatrix_->pixelToRobotWithZ(circle.center, fieldDimensions_->ballDiameter / 2, pos);
        ballData_->positions.push_back(pos);
        ballData_->imagePositions.push_back(circle);
      }
    }
    ballData_->timestamp = imageData_->timestamp;
  }
  sendDebugImage();
}

std::vector<Circle<int>> BallDetectionNeuralNet::mergeSeeds(const std::vector<Circle<int>>& seeds)
{
  Chronometer time(debug(), mount_ + ".cycle_time.merge_seeds");
  // getSeed writes the found seed into its argument
  std::vector<MergedCandidate> mergedCandidates;
  for (auto& candidate : seeds)
  {
    bool merged = false;
    for (auto& mergedCandidate : mergedCandidates)
    {
      const int maxRadius =
          std::max(candidate.radius,
                   static_cast<int>(mergedCandidate.candidate.radius / mergedCandidate.count));
      if ((mergedCandidate.candidate.center / mergedCandidate.count - candidate.center).norm() <
          maxRadius * 2)
      {
        mergedCandidate.candidate.center += candidate.center;
        mergedCandidate.candidate.radius += candidate.radius;
        mergedCandidate.count++;
        merged = true;
        break;
      }
    }
    if (!merged)
    {
      const MergedCandidate newCandidate = {candidate, 1};
      mergedCandidates.push_back(newCandidate);
    }
  }
  std::vector<Circle<int>> result;
  for (const auto& c : mergedCandidates)
  {
    if (c.count >= candidateMinSeeds_())
    {
      result.emplace_back(c.candidate.center / c.count, c.candidate.radius / c.count);
    }
  }
  Circle<int> foundBall;
  if (projectFoundBall(foundBall))
  {
    ballData_->filteredProjectedBall = foundBall;
    result.push_back(foundBall);
  }
  return result;
}

bool BallDetectionNeuralNet::applyFilter(const std::vector<Circle<int>>& candidates,
                                         std::vector<Circle<int>>& best)
{
  debugCircles_.clear();
  debugCircles_.reserve(candidates.size() + 1);
  std::vector<Circle<int>> balls;

  float bestResult = -1.f;
  for (auto& circle : candidates)
  {
    // sample the bounding box of the candidate
    std::array<dim_t, 3> colorSampledDim;
    std::vector<float> colorSampled;
    const bool ok = sampleBoundingBox(circle, netSampleSize_, colorSampled, colorSampledDim);
    if (!ok)
    {
      // if there is not enough black in the sampled candidate, drop it
      debugCircles_.emplace_back(circle, Color::BLUE);
      continue;
    }
    // inference the CNN, drop if it's not in the ball class
    float cnnResult = filterByCNN(colorSampled, colorSampledDim);
    if (cnnResult <= 0.f)
    {
      debugCircles_.emplace_back(circle, Color::WHITE);
      continue;
    }
    debugCircles_.emplace_back(circle, Color::ORANGE);
    // if there is no ball yet, this is the best result
    if (best.size() == 0)
    {
      best.push_back(circle);
      bestResult = cnnResult;
      continue;
    }
    // check if this ball is near the current best result
    bool nearBest = best.size() > 0;
    for (auto& currentBest : best)
    {
      nearBest = nearBest && (currentBest.center - circle.center).norm() < currentBest.radius;
    }
    // add the current result if it is near the current best result
    if (nearBest)
    {
      best.push_back(circle);
      if (cnnResult > bestResult)
      {
        bestResult = cnnResult;
      }
      continue;
    }
    // if it's not near the current best result but has better classification, replace the current
    // best result
    if (bestResult < cnnResult)
    {
      best.clear();
      best.push_back(circle);
      bestResult = cnnResult;
    }
  }
  if (bestResult > -1.f)
  {
    for (auto& ball : best)
    {
      debugCircles_.emplace_back(ball, Color::GREEN);
    }
    return true;
  }
  return false;
}

void BallDetectionNeuralNet::pool(const std::vector<float>& img, const std::array<dim_t, 3>& inDim,
                                  const int poolType, std::vector<float>& result,
                                  std::array<dim_t, 3>& outDim) const
{
  switch (poolType)
  {
    case 0:
      result = img;
      outDim = inDim;
      return;
    case 1:
      maxPool2x2(img, inDim, result, outDim);
      return;
    case 2:
      avgPool2x2(img, inDim, result, outDim);
      return;
    default:
      assert(false); // Unknown pooling type!
  }
}


void BallDetectionNeuralNet::normalize(std::vector<float>& img,
                                       const std::vector<std::vector<float>>& norm) const
{
  assert(norm.size() == 4);
  for (dim_t i = 0; i < img.size(); i++)
  {
    const float mean = norm[0][i];
    const float var = norm[1][i];
    const float scale = norm[2][i];
    const float beta = norm[3][i];
    img[i] = (scale * (img[i] - mean)) / (var + 1e-3) + beta;
  }
}


void BallDetectionNeuralNet::maxPool2x2(const std::vector<float>& img,
                                        const std::array<dim_t, 3>& inDim,
                                        std::vector<float>& result,
                                        std::array<dim_t, 3>& outDim) const
{
  outDim = {{(inDim[0] + 1) / 2, (inDim[1] + 1) / 2, inDim[2]}};
  result.resize(outDim[0] * outDim[1] * outDim[2]);
  for (dim_t i = 0; i < outDim[0]; i++)
  {
    const dim_t inIPos1 = i * 2 * inDim[1] * inDim[2];
    const dim_t inIPos2 = (i * 2 + 1) * inDim[1] * inDim[2];
    for (dim_t j = 0; j < outDim[1]; j++)
    {
      const dim_t inJPos1 = j * 2 * inDim[2];
      const dim_t inJPos2 = (j * 2 + 1) * inDim[2];
      for (dim_t k = 0; k < outDim[2]; k++)
      {
        const dim_t inPos1 = inIPos1 + inJPos1 + k;
        const dim_t inPos2 = inIPos1 + inJPos2 + k;
        const dim_t inPos3 = inIPos2 + inJPos1 + k;
        const dim_t inPos4 = inIPos2 + inJPos2 + k;

        const dim_t outPos = i * outDim[1] * outDim[2] + j * outDim[2] + k;

        const float val1 = img[inPos1];
        const bool j_ok = j * 2 + 1 < inDim[1];
        const bool i_ok = i * 2 + 1 < inDim[0];
        const float val2 = j_ok ? img[inPos2] : 0;
        const float val3 = i_ok ? img[inPos3] : 0;
        const float val4 = i_ok && j_ok ? img[inPos4] : 0;

        result[outPos] = std::max(std::max(std::max(val1, val2), val3), val4);
      }
    }
  }
}

void BallDetectionNeuralNet::avgPool2x2(const std::vector<float>& img,
                                        const std::array<dim_t, 3>& inDim,
                                        std::vector<float>& result,
                                        std::array<dim_t, 3>& outDim) const
{
  outDim = {{(inDim[0] + 1) / 2, (inDim[1] + 1) / 2, inDim[2]}};
  result.resize(outDim[0] * outDim[1] * outDim[2]);
  for (dim_t i = 0; i < outDim[0]; i++)
  {
    const dim_t inIPos1 = i * 2 * inDim[1] * inDim[2];
    const dim_t inIPos2 = (i * 2 + 1) * inDim[1] * inDim[2];
    for (dim_t j = 0; j < outDim[1]; j++)
    {
      const dim_t inJPos1 = j * 2 * inDim[2];
      const dim_t inJPos2 = (j * 2 + 1) * inDim[2];
      for (dim_t k = 0; k < outDim[2]; k++)
      {
        const dim_t inPos1 = inIPos1 + inJPos1 + k;
        const dim_t inPos2 = inIPos1 + inJPos2 + k;
        const dim_t inPos3 = inIPos2 + inJPos1 + k;
        const dim_t inPos4 = inIPos2 + inJPos2 + k;

        const dim_t outPos = i * outDim[1] * outDim[2] + j * outDim[2] + k;
        result[outPos] = (img[inPos1] + img[inPos2] + img[inPos3] + img[inPos4]) / 4;
      }
    }
  }
}

void BallDetectionNeuralNet::convolution(
    const std::vector<float>& input, const std::array<dim_t, 3>& inDim,
    const std::vector<std::vector<std::vector<std::vector<float>>>>& mask,
    const std::vector<float>& bias, const int activation, const int pooling,
    std::vector<float>& output, std::array<dim_t, 3>& outDim) const
{
  assert(mask[0][0].size() == inDim[2]);
  const std::array<dim_t, 3> convDim = {
      {(inDim[0]), (inDim[1]), static_cast<dim_t>(mask[0][0][0].size())}};
  if (convDim[2] == 0)
  {
    output = input;
    outDim = inDim;
    return;
  }
  std::vector<float> conv(convDim[0] * convDim[1] * convDim[2]);
  const dim_t padI = (mask.size() - 1) / 2;
  const dim_t padJ = (mask[0].size() - 1) / 2;
  for (dim_t i = 0; i < inDim[0]; i++)
  {
    for (dim_t j = 0; j < inDim[1]; j++)
    {
      for (dim_t k = 0; k < convDim[2]; k++)
      {
        const dim_t outPos = i * convDim[1] * convDim[2] + j * convDim[2] + k;
        conv[outPos] = bias[k];
        for (dim_t di = 0; di < mask.size(); di++)
        {
          for (dim_t dj = 0; dj < mask[di].size(); dj++)
          {
            for (dim_t q = 0; q < mask[di][dj].size(); q++)
            {
              const dim_t ci = i + di - padI;
              const dim_t cj = j + dj - padJ;
              if (inDim[0] > ci && inDim[1] > cj)
              {
                const dim_t inPos = ci * inDim[1] * inDim[2] + cj * inDim[2] + q;
                conv[outPos] += input[inPos] * mask[di][dj][q][k];
              }
            }
          }
        }
      }
    }
  }
  pool(conv, convDim, pooling, output, outDim);
  activate(output, activation);
}

void BallDetectionNeuralNet::activate(std::vector<float>& img, const int activation) const
{
  if (activation == 0) // tanh
  {
    for (dim_t i = 0; i < img.size(); i++)
    {
      img[i] = std::tanh(img[i]);
    }
  }
  else if (activation == 1) // relu
  {
    for (dim_t i = 0; i < img.size(); i++)
    {
      if (img[i] < 0)
      {
        img[i] = 0;
      }
    }
  }
}

void BallDetectionNeuralNet::execLayer(const std::vector<float>& input,
                                       const std::vector<std::vector<float>>& weights,
                                       const std::vector<float>& bias, const int activation,
                                       std::vector<float>& output) const
{
  assert(output.size() == bias.size());
  assert(weights.size() == input.size());
  for (dim_t i = 0; i < output.size(); i++)
  {
    output[i] = bias[i];
    for (dim_t j = 0; j < weights.size(); j++)
    {
      assert(weights[j].size() == output.size());
      output[i] += weights[j][i] * input[j];
    }
  }
  activate(output, activation);
}


bool BallDetectionNeuralNet::sampleBoundingBox(const Circle<int>& circle, const dim_t sampleSize,
                                               std::vector<float>& colorSampled,
                                               std::array<dim_t, 3>& colorSampledDim) const
{
  const Vector2i from(circle.center.x() * 2 - circle.radius, circle.center.y() - circle.radius);
  const float scale = circle.radius * 2.0f / sampleSize;

  colorSampledDim = {{sampleSize, sampleSize, 3}};
  colorSampled.resize(sampleSize * sampleSize * 3);

  unsigned int numDark = 0;
  Vector2i pixel(from);
  for (dim_t y = 0; y < sampleSize; y++)
  {
    pixel.y() = from.y() + static_cast<int>(y * scale);
    for (dim_t x = 0; x < sampleSize; x++)
    {
      // First, calculate x position in YUV444 coords
      pixel.x() = from.x() + static_cast<int>(x * scale);
      // Check if 444 coord is even
      const bool xEven = pixel.x() % 2 == 0;
      // Convert to 422 coordinate
      pixel.x() /= 2;
      // Calculate coordinate in sampled array
      const dim_t pos = y * sampleSize * 3 + x * 3;
      // Fallback to 128 if pixel is not inside image
      if (!imageData_->image422.isInside(pixel))
      {
        const float fallback = scaleByte(128);
        colorSampled[pos] = fallback;
        colorSampled[pos + 1] = fallback;
        colorSampled[pos + 2] = fallback;
        continue;
      }
      // Get 422 Color
      const YCbCr422& color = imageData_->image422[pixel];
      // If 444 coord was even, take the first y value. Otherwise the second
      const std::uint8_t& yByte = xEven ? color.y1_ : color.y2_;
      colorSampled[pos] = scaleByte(yByte);
      colorSampled[pos + 1] = scaleByte(color.cb_);
      colorSampled[pos + 2] = scaleByte(color.cr_);
      if (yByte < seedDark_())
      {
        numDark++;
      }
    }
  }
  return static_cast<float>(numDark) / (sampleSize * sampleSize) >= 0.1f;
}

float BallDetectionNeuralNet::filterByCNN(std::vector<float>& sampled,
                                          std::array<dim_t, 3>& colorSampledDim)
{
  const Chronometer time(debug(), mount_ + ".cycle_time.net");

  assert(netConvActivation_.size() == netConvMask_.size());
  assert(netConvActivation_.size() == netConvBias_.size());
  assert(netConvActivation_.size() == netConvActivation_.size());
  assert(netConvActivation_.size() == netConvPooling_.size());
  std::vector<float>& convOut = sampled;
  std::array<dim_t, 3>& convOutDim = colorSampledDim;
  for (dim_t convLayer = 0; convLayer < netConvActivation_.size(); convLayer++)
  {
    std::vector<float> conv;
    std::array<dim_t, 3> convDim;
    convolution(convOut, convOutDim, netConvMask_[convLayer], netConvBias_[convLayer],
                netConvActivation_[convLayer], netConvPooling_[convLayer], conv, convDim);
    convOut = conv;
    convOutDim = convDim;
  }

  normalize(convOut, netNorm_);

  const std::vector<std::vector<std::vector<float>>>& w = netFCWeights_;
  const std::vector<std::vector<float>>& b = netFCBias_;
  std::vector<float>& x = convOut;
  assert(w.size() == b.size());
  for (dim_t i = 0; i < w.size(); i++)
  {
    std::vector<float> out(b[i].size());
    execLayer(x, w[i], b[i], netFCActivation_, out);
    x = out;
  }
  assert(x.size() == 2);
  return x[0] - x[1] - netAccuracy_();
}

std::vector<Circle<int>> BallDetectionNeuralNet::getSeeds()
{
  std::vector<Circle<int>> seeds;
  debugSeeds_.clear();
  for (auto& scanline : imageSegments_->verticalScanlines)
  {
    unsigned long regionCount = scanline.segments.size();
    for (unsigned int i = 0; i < regionCount; i++)
    {
      if (scanline.segments[i].ycbcr422.y1_ > seedDark_())
      {
        continue;
      }
      if (!fieldBorder_->isInsideField(scanline.segments[i].start))
      {
        continue;
      }
      const Vector2i seed = (scanline.segments[i].start + scanline.segments[i].end) / 2;
      int pixelRadius = 0;
      cameraMatrix_->getPixelRadius(imageData_->image422.size, seed,
                                    fieldDimensions_->ballDiameter / 2, pixelRadius);

      const float regionSize =
          static_cast<float>(scanline.segments[i].end.y() - scanline.segments[i].start.y()) /
          pixelRadius;
      if (regionSize < seedRadiusRatioMin_() || regionSize > seedRadiusRatioMax_())
      {
        continue;
      }
      const std::array<Vector2i, 8> directions = {
          {{-1, -2}, {0, -2}, {1, -2}, {-1, 0}, {1, 0}, {-1, 2}, {0, 2}, {1, 2}}};

      int seedY = imageData_->image422[seed].y1_;
      bool allBrighter = true;
      int score = 0;
      for (auto& d : directions)
      {
        // Move from seed into direction * pixelRadius * (10/25)
        // 10/25 is a well working magic number
        // 422 conversion is done by multiplying d.y with two (see above) and dividing the magic
        // number
        const Vector2i& point = seed + (d * pixelRadius * 5 / 25);
        if (!imageData_->image422.isInside(point))
        {
          continue;
        }
        const int pointY = imageData_->image422[point].y1_;
        if (pointY - seedY < seedBrightMin_())
        {
          allBrighter = false;
          break;
        }
        if (pointY - seedY > seedBright_())
        {
          score++;
        }
      }

      if (!allBrighter)
      {
        continue;
      }

      if (score < seedBrightScore_())
      {
        continue;
      }
      seeds.emplace_back(seed, pixelRadius);
      debugSeeds_.emplace_back(seed, pixelRadius);
    }
  }
  return seeds;
}

void BallDetectionNeuralNet::sendDebugImage()
{
  const std::string debugImageMount = mount_ + "." + imageData_->identification + "_image";

  if (debug().isSubscribed(debugImageMount))
  {
    Image debugImage(imageData_->image422.to444Image());

    for (auto& seed422 : debugSeeds_)
    {
      const Circle<int> seed(Image422::get444From422Vector(seed422.center), seed422.radius);
      const int radiusHalf = seed.radius * 10 / 25;
      debugImage.line(Vector2i(seed.center.x() - radiusHalf, seed.center.y()),
                      Vector2i(seed.center.x() + radiusHalf, seed.center.y()), Color::BLUE);
      debugImage.line(Vector2i(seed.center.x(), seed.center.y() - radiusHalf),
                      Vector2i(seed.center.x(), seed.center.y() + radiusHalf), Color::BLUE);
      debugImage.line(Vector2i(seed.center.x() - radiusHalf, seed.center.y() - radiusHalf),
                      Vector2i(seed.center.x() + radiusHalf, seed.center.y() + radiusHalf),
                      Color::BLUE);
      debugImage.line(Vector2i(seed.center.x() + radiusHalf, seed.center.y() - radiusHalf),
                      Vector2i(seed.center.x() - radiusHalf, seed.center.y() + radiusHalf),
                      Color::BLUE);
    }
    for (auto& debugCircle : debugCircles_)
    {
      Circle<int> circle(debugCircle.circle);
      circle.from422to444();
      debugImage.cross(circle.center, 3, debugCircle.color);
      debugImage.circle(circle.center, circle.radius, debugCircle.color);
    }
    Circle<int> foundBall;
    if (projectFoundBall(foundBall))
    {
      foundBall.from422to444();
      debugImage.rectangle(foundBall.center.array() - foundBall.radius,
                           foundBall.center.array() + foundBall.radius, Color::BLACK);
    }
    debug().sendImage(debugImageMount, debugImage);
  }
  if (writeCandidatesToDisk_() && gameControllerState_->gameState == GameState::PLAYING &&
      gameControllerState_->penalty == Penalty::NONE)
  {
    PngConverter img_conv_;
    for (auto& circle : debugCircles_)
    {
      const Vector2i from(circle.circle.center.x() - circle.circle.radius / 2,
                          circle.circle.center.y() - circle.circle.radius);
      const Vector2i to(circle.circle.center.x() + circle.circle.radius / 2,
                        circle.circle.center.y() + circle.circle.radius);
      Image422 ball_candidate(to - from);
      for (int x = 0; x < to.x() - from.x(); x++)
      {
        for (int y = 0; y < to.y() - from.y(); y++)
        {
          Vector2i point = Vector2i(x, y);
          if (imageData_->image422.isInside(from + point))
          {
            ball_candidate[point] = imageData_->image422[from + point];
          }
          else
          {
            const uint8_t fallback = 128;
            ball_candidate[point] = YCbCr422(fallback, fallback, fallback, fallback);
          }
        }
      }
      std::ofstream fs;
      CVData image;
      img_conv_.convert(ball_candidate.to444Image(), image);
      std::string fn = mount_;
      if (circle.color == Color::ORANGE)
      {
        fn += ".true";
      }
      else if (circle.color == Color::WHITE)
      {
        fn += ".false";
      }
      else
      {
        continue;
      }
      fn = robotInterface().getFileRoot() + "filetransport_ball_candidates/" + fn + "." +
           std::to_string(candidateCount_++) + ".png";
      fs.open(fn, std::ios_base::out | std::ios_base::trunc | std::ios_base::binary);
      fs.write((const char*)image.data(), image.size());
      fs.close();
    }
  }
}

bool BallDetectionNeuralNet::projectFoundBall(Circle<int>& foundBall) const
{
  if (projectFoundBalls_() && ballState_->found)
  {
    Vector2f position = ballState_->position;
    if (ballState_->moved)
    {
      position += ballState_->velocity * cycleInfo_->cycleTime;
    }
    cameraMatrix_->robotWithZToPixel(
        Vector3f(position.x(), position.y(), fieldDimensions_->ballDiameter / 2), foundBall.center);
    if (imageData_->image422.isInside(foundBall.center) &&
        fieldBorder_->isInsideField(foundBall.center))
    {
      cameraMatrix_->getPixelRadius(imageData_->image422.size, foundBall.center,
                                    fieldDimensions_->ballDiameter / 2, foundBall.radius);
      return true;
    }
  }
  return false;
}
