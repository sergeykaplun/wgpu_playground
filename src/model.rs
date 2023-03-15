// //use gltf::Buffer;

// use gltf::{mesh::BoundingBox, Node};


// struct Model {
//     //vertice: Buffer,
//     //indices: Buffer,
    
//     aabb: glm::Mat4,
    
//     nodes: Vec<Box<Node>>,
//     linearNodes: Vec<Box<Node>>,

// }

// struct Node {
//     parent:         Box<Node>,
//     index:          u32,
//     children:       Vec<Box<Node>>,
//     name:           String,

    
//     // Mesh *mesh;
//     // Skin *skin;

//     skinIndex:      i32,             //TODO = -1;

//     matrix:         glm::Mat4,
//     translation:    glm::Vec3,
//     scale:          glm::Vec3,
//     rotation:       glm::Quat,
//     bvh:            BoundingBox,
//     aabb:           BoundingBox,
    
//     // glm::mat4 localMatrix();
//     // glm::mat4 getMatrix();
//     // void update();
//     // ~Node();
// }