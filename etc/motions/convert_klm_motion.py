import json
import math


def transform_devils_angle(devils_angle):
    if devils_angle.endswith('deg'):
        devils_angle = devils_angle[:-3]
    return (float(devils_angle) / 180) * math.pi


def extract_angles(line):
    return line.split('[')[1].split(']')[0].split(',')


def convert_angles(head_angles, arm_angles, leg_angles):
    return {
        'head': {
            'yaw': head_angles[0],
            'pitch': head_angles[1],
        },
        'left_arm': {
            'shoulder_pitch': arm_angles[0],
            'shoulder_roll': arm_angles[1],
            'elbow_yaw': arm_angles[2],
            'elbow_roll': arm_angles[3],
            'wrist_yaw': arm_angles[4],
            'hand': arm_angles[5],
        },
        'right_arm': {
            'shoulder_pitch': arm_angles[6],
            'shoulder_roll': arm_angles[7],
            'elbow_yaw': arm_angles[8],
            'elbow_roll': arm_angles[9],
            'wrist_yaw': arm_angles[10],
            'hand': arm_angles[11],
        },
        'left_leg': {
            'hip_yaw_pitch': leg_angles[0],
            'hip_roll': leg_angles[1],
            'hip_pitch': leg_angles[2],
            'knee_pitch': leg_angles[3],
            'ankle_pitch': leg_angles[4],
            'ankle_roll': leg_angles[5],
        },
        'right_leg': {
            'hip_yaw_pitch': leg_angles[6],
            'hip_roll': leg_angles[7],
            'hip_pitch': leg_angles[8],
            'knee_pitch': leg_angles[9],
            'ankle_pitch': leg_angles[10],
            'ankle_roll': leg_angles[11],
        }
    }


motion_file = {'initial_positions': None, 'frames': []}
with open('standUpBackNaoFast.kfm') as f:
    head_angles = None
    arm_angles = None
    leg_angles = None
    for line in f:
        stripped_line = line.strip()
        angles_type = None
        if stripped_line.startswith('headAngles'):
            angles_type = 'head'
        elif stripped_line.startswith('armsAngles'):
            angles_type = 'arms'
        elif stripped_line.startswith('legsAngles'):
            angles_type = 'legs'
        if angles_type is not None:
            devils_angles = extract_angles(stripped_line)
            radian_angles = [transform_devils_angle(angle) for angle in devils_angles]
            if angles_type == 'head':
                head_angles = radian_angles
            elif angles_type == 'arms':
                arm_angles = radian_angles
            elif angles_type == 'legs':
                leg_angles = radian_angles
        if stripped_line.startswith('duration'):
            duration = float(stripped_line.split(' ')[2][:-1]) / 1000
            if motion_file['initial_positions'] is None:
                motion_file['initial_positions'] = convert_angles(head_angles, arm_angles, leg_angles)
            else:
                motion_file['frames'].append({
                    'duration': duration,
                    'positions': convert_angles(head_angles, arm_angles, leg_angles),
                })

print(json.dumps(motion_file, indent=2))
