import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import Stats from 'three/addons/libs/stats.module.js';

function setupLighting(scene, sceneData) {
    const ambientLight = new THREE.AmbientLight(0x404040); // soft white light
    scene.add(ambientLight);
    const hemiLight = new THREE.HemisphereLight(0xffffff, 0x8d8d8d, 2);
    hemiLight.position.set(0, 20, 0);
    scene.add(hemiLight);

    for (const light of sceneData.lights) {
        const pointLight = new THREE.PointLight(0xffffff, 1);
        pointLight.position.fromArray(light.pos);
        scene.add(pointLight);
    }
}

function setupGround(scene) {
    const mesh = new THREE.Mesh(new THREE.PlaneGeometry(100, 100), new THREE.MeshPhongMaterial({ color: 0xcbcbcb, depthWrite: false }));
    // mesh.rotation.x = - Math.PI / 2;
    mesh.receiveShadow = true;
    scene.add(mesh);
}

function loadMeshes(sceneData) {
    const bodyMeshes = {};
    for (const [meshName, meshData] of Object.entries(sceneData.meshes)) {
        const geometry = new THREE.BufferGeometry();
        geometry.setAttribute("position", new THREE.Float32BufferAttribute(meshData.vertices.flat(), 3));
        geometry.setIndex(meshData.faces.flat());
        geometry.computeVertexNormals();

        const mesh = new THREE.Mesh(geometry);
        bodyMeshes[meshName] = mesh;
    }
    return bodyMeshes;
}

function setupMeshGroups(scene, sceneData, bodyMeshes) {
    const bodyGroups = {};
    for (const [bodyName, bodyData] of Object.entries(sceneData.bodies)) {
        const group = new THREE.Group();
        group.name = bodyName;
        scene.add(group);
        bodyGroups[bodyName] = group;

        for (const geom of bodyData.geoms) {
            if (!geom.mesh) continue;
            const mesh = bodyMeshes[geom.mesh].clone();
            mesh.material = new THREE.MeshStandardMaterial({ color: geom.rgba });

            mesh.position.fromArray(geom.pos);
            mesh.quaternion.set(geom.quat[1], geom.quat[2], geom.quat[3], geom.quat[0]); // convert [w,x,y,z] -> [x,y,z,w]

            group.add(mesh);
        }
    }
    return bodyGroups;
}

async function init() {
    // ======= 1. Setup Three.js =======
    const scene = new THREE.Scene();
    THREE.Object3D.DEFAULT_UP.set(0, 0, 1);
    const camera = new THREE.PerspectiveCamera(60, window.innerWidth / window.innerHeight, 0.01, 100);
    camera.up.set(0, 0, 1);
    camera.position.set(1, 1, 1);
    camera.lookAt(0, 0, 0);
    scene.add(new THREE.AxesHelper(1));
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(window.innerWidth, window.innerHeight);
    document.body.appendChild(renderer.domElement);
    new OrbitControls(camera, renderer.domElement);
    const stats = new Stats();
    document.getElementById("stats").appendChild(stats.dom);

    scene.background = new THREE.Color(0xa0a0a0);
    scene.fog = new THREE.Fog(0xa0a0a0, 10, 50);

    window.addEventListener('resize', () => {
        camera.aspect = window.innerWidth / window.innerHeight;
        camera.updateProjectionMatrix();
        renderer.setSize(window.innerWidth, window.innerHeight);
    });

    const response = await fetch("http://localhost:8000/scene");
    const sceneData = await response.json();
    
    setupGround(scene);
    setupLighting(scene, sceneData);
    const bodyMeshes = loadMeshes(sceneData);
    const bodyGroups = setupMeshGroups(scene, sceneData, bodyMeshes);

    const ws = new WebSocket("ws://localhost:8000/scene/subscribe");
    ws.onmessage = (event) => {
        const state = JSON.parse(event.data);
        for (const [name, t] of Object.entries(state.bodies)) {
            const group = bodyGroups[name];
            if (!group) continue;
            group.position.fromArray(t.pos);
            group.quaternion.set(t.quat[1], t.quat[2], t.quat[3], t.quat[0]);
        }
        stats.update();
    };

    function animate() {
        requestAnimationFrame(animate);
        renderer.render(scene, camera);
    }
    animate();
}

init();