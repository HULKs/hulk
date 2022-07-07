import { useEffect, useState } from 'react'
import Connection, { Cycler, OutputType } from '../Connection/Connection'
import './BallCandidates.css'

export default function BallCandidates({
  selector,
  connector,
  connection,
  cycler,
}: {
  selector: JSX.Element
  connector: JSX.Element
  connection: Connection | null
  cycler: Cycler
}) {
  const [imageData, setImageData] = useState<Blob | undefined>(undefined)
  const [ballCandidatesData, setBallCandidatesData] = useState<
    | Array<{
      candidate_circle: {
        radius: number
        center: Array<number>
      }
      corrected_circle: {
        radius: number
        center: Array<number>
      }
      classifier_confidence: number
      preclassifier_confidence: number
      merge_weight: number
    }>
    | null
    | undefined
  >(undefined)
  const [filteredBallsData, setFilteredBallsData] = useState<Array<{ radius: number, center: Array<number> }> | null | undefined>(undefined);
  const [ballsData, setBallsData] = useState<
    | Array<{
      image_location: {
        radius: number
        center: Array<number>
      }
      position: Array<number>
    }>
    | null
    | undefined
  >(undefined)
  useEffect(() => {
    if (connection === null) {
      return
    }
    const unsubscribeImage = connection.subscribeImage(
      cycler,
      (data) => {
        setImageData(data)
      },
      (error) => {
        alert(`Error: ${error}`)
      },
    )
    const unsubscribeBallCandidates = connection.subscribeOutput(
      cycler,
      OutputType.Additional,
      'ball_candidates',
      (data) => {
        setBallCandidatesData(data)
      },
      (error) => {
        alert(`Error: ${error}`)
      },
    )
    const unsubscribeFilteredBalls = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Additional,
      cycler === Cycler.VisionTop ? 'filtered_balls_in_image_top' : 'filtered_balls_in_image_bottom',
      (data) => {
        setFilteredBallsData(data)
      },
      (error) => {
        alert(`Error: ${error}`)
      },
    )
    const unsubscribeBalls = connection.subscribeOutput(
      cycler,
      OutputType.Main,
      'balls',
      (data) => {
        setBallsData(data)
      },
      (error) => {
        alert(`Error: ${error}`)
      },
    )
    return () => {
      unsubscribeImage()
      unsubscribeBallCandidates()
      unsubscribeFilteredBalls()
      unsubscribeBalls()
    }
  }, [connection, cycler])
  const [imageUrl, setImageUrl] = useState<string | undefined>(undefined)
  useEffect(() => {
    if (imageData !== undefined) {
      const imageUrl = URL.createObjectURL(imageData)
      setImageUrl(imageUrl)
      return () => {
        URL.revokeObjectURL(imageUrl)
      }
    }
  }, [imageData])
  const candidateCircles =
    ballCandidatesData !== undefined && ballCandidatesData !== null
      ? ballCandidatesData.map((candidate, index) => {
        const corrected_circle =
          candidate.corrected_circle !== null ? (
            <circle
              cx={candidate.corrected_circle.center[0]}
              cy={candidate.corrected_circle.center[1]}
              r={candidate.corrected_circle.radius}
              stroke="green"
              stroke-width="3"
              fill="none"
            />
          ) : null
        return (
          <g key={index}>
            <g
              transform={`translate(
                ${candidate.candidate_circle.center[0] -
                candidate.candidate_circle.radius
                }
                ${candidate.candidate_circle.center[1] -
                candidate.candidate_circle.radius
                }
              )`}
            >
              <circle
                cx={candidate.candidate_circle.radius}
                cy={candidate.candidate_circle.radius}
                r={candidate.candidate_circle.radius}
                stroke="blue"
                stroke-width="3"
                fill="none"
              />
              <text x="0" y="-3" fontSize="10">
                {candidate.preclassifier_confidence.toFixed(2)}
              </text>
            </g>
            {corrected_circle}
          </g>
        )
      })
      : null
  const balls =
    ballsData !== undefined && ballsData !== null
      ? ballsData.map((ball, index) => {
        return (
          <g key={index}>
            <g
              transform={`translate(
                ${ball.image_location.center[0] - ball.image_location.radius}
                ${ball.image_location.center[1] - ball.image_location.radius}
              )`}
            >
              <circle
                cx={ball.image_location.radius}
                cy={ball.image_location.radius}
                r={ball.image_location.radius}
                stroke="white"
                stroke-width="1"
                fill="none"
              />
            </g>
          </g>
        )
      })
      : null
  const filtered_balls = filteredBallsData !== undefined && filteredBallsData !== null ? filteredBallsData.map((ball, index) => {
    return (
      <g key={index}>
        <g
          transform={`translate(
                ${ball.center[0] - ball.radius}
                ${ball.center[1] - ball.radius}
              )`}
        >
          <circle
            cx={ball.radius}
            cy={ball.radius}
            r={ball.radius}
            stroke="red"
            stroke-width="1"
            fill="none"
          />
        </g>
      </g>
    )
  }) : null;
  return (
    <div className="ballCandidates">
      <div className="header">
        <div className="panelType">BallCandidates:</div>
        <div className="cycler">{cycler}</div>
        {selector}
        {connector}
      </div>
      {imageUrl !== undefined ? (
        <>
          <img className="content" src={imageUrl} alt="" />
          <svg className="overlay" viewBox="0 0 640 480">
            {candidateCircles}
            {balls}
            {filtered_balls}
          </svg>
        </>
      ) : (
        <div className="content noData">NAO has not sent any data yet</div>
      )}
    </div>
  )
}
