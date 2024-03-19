const MAX_EROSION_STEPS = 500;
const DROPLET_ARRAY_SIZE = 64 * 16;
@group(0) @binding(0)
var<storage, read_write> droplets: array<Droplet, DROPLET_ARRAY_SIZE>;
@group(0) @binding(1)
var<storage, read_write> results: array<f32>;
@group(0) @binding(2)
var<uniform> image_size: vec2<u32>;
@group(0) @binding(3)
var<uniform> world_size: vec2<u32>;

struct Droplet {
    position_x: u32,
    position_y: u32,
    radius: u32,
    sediment: f32,
    water: f32,
    speed: f32,
    direction_x: f32,
    direction_y: f32,
}

const pi = 3.14159265359;
const erosion_speed: f32 = 0.2;
const gravity: f32 = 20.0;
const deposition_speed: f32 = 0.2;
const water_evaporation_speed: f32 = 0.0001;
const minimum_slope: f32 = 0.01;
const direction_inertia: f32 = 3.0;
const carry_capacity_modifier: f32 = 1.0;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let droplet_index = invocation_id.x;
    erosion(droplet_index);
}

fn erosion(droplet_position: u32) {
    for (var i: i32 = 0; i < MAX_EROSION_STEPS; i++) {
        let position = vec2<u32>(droplets[droplet_position].position_x, droplets[droplet_position].position_y);
        let radius = i32(droplets[droplet_position].radius);
        //get the neighbours with the current radius
        for (var dx: i32 = -radius; dx <= radius; dx++) {
                for (var dy: i32 = -radius; dy <= radius; dy++) {
                    let neighbour = vec2<i32>(i32(position.x) + dx, i32(position.y) + dy);
                    if bound_check_i32(neighbour) && distance(vec2<f32>(position), vec2<f32>(neighbour)) < f32(radius) {
                        if all(neighbour == vec2<i32>(position)) {
                            continue;
                        }
                        var direction = vec2<f32>(position) - vec2<f32>(neighbour);
                        let height_difference = results[position.x + position.y * image_size.x] - results[neighbour.x + neighbour.y * i32(image_size.x)];
                        direction *= (-height_difference) * gravity * direction_inertia;
                        droplets[droplet_position].direction_x += direction.x;
                        droplets[droplet_position].direction_y += direction.y;
                    }
                }
            }
        let normalized_direction = normalize(vec2<f32>(droplets[droplet_position].direction_x, droplets[droplet_position].direction_y));
        droplets[droplet_position].direction_x = normalized_direction.x;
        droplets[droplet_position].direction_y = normalized_direction.y;

        let next_position = vec2<u32>(round(vec2<f32>(position) + (normalized_direction * f32(radius))));

        if !bound_check_u32(next_position) {
            return;
        }

        droplets[droplet_position].water *= 1.0 - water_evaporation_speed;

        let height_difference = results[position.x + position.y * image_size.x] - results[next_position.x + next_position.y * image_size.x];
        droplets[droplet_position].speed += height_difference * gravity;
        droplets[droplet_position].direction_x *= droplets[droplet_position].speed;
        droplets[droplet_position].direction_y *= droplets[droplet_position].speed;

        let carry_capacity = max(height_difference, minimum_slope) * droplets[droplet_position].speed * droplets[droplet_position].water * f32(droplets[droplet_position].radius) *  carry_capacity_modifier;

        if droplets[droplet_position].water < 0.1 || droplets[droplet_position].speed < 0.0 {
            droplets[droplet_position].radius *= u32(5);
            deposit(droplet_position, droplets[droplet_position].sediment * deposition_speed);
            return;
        }
        //else if height_difference < 0.0 {
        //    let deposit_amount = min(-height_difference, droplets[droplet_position].sediment);
        //    let temp_radius = droplets[droplet_position].radius;
        //    droplets[droplet_position].radius = u32(1);
        //    deposit(droplet_position, deposit_amount);
        //    droplets[droplet_position].radius = temp_radius;
        //}
        else if droplets[droplet_position].sediment > carry_capacity {
            let deposit_amount = (droplets[droplet_position].sediment  - carry_capacity) * deposition_speed;
            deposit(droplet_position, deposit_amount);
        }
        else {
            let sediment_delta = (carry_capacity - droplets[droplet_position].sediment) * erosion_speed;
            let erosion_amount = min(sediment_delta, height_difference);
            erode(droplet_position, erosion_amount);
        }
        droplets[droplet_position].position_x = next_position.x;
        droplets[droplet_position].position_y = next_position.y;
    }
    droplets[droplet_position].radius *= u32(5);
    deposit(droplet_position, droplets[droplet_position].sediment * deposition_speed);
    return;
}

fn deposit(droplet_position: u32, amount: f32) {
    let radius = i32(droplets[droplet_position].radius);
    let position = vec2<u32>(droplets[droplet_position].position_x, droplets[droplet_position].position_y);
    for (var dx: i32 = -radius; dx <= radius; dx++) {
        for (var dy: i32 = -radius; dy <= radius; dy++) {
            let neighbour = vec2<i32>(i32(position.x) + dx, i32(position.y) + dy);
                if bound_check_i32(neighbour) && distance(vec2<f32>(position), vec2<f32>(neighbour)) < f32(radius){
                    let distance = distance(vec2<f32>(position), vec2<f32>(neighbour));
                    let deposit_amount = amount * normal_curve(0.0, f32(radius) * 0.5, f32(distance));
                    results[neighbour.x + neighbour.y * i32(image_size.x)] = clamp(results[neighbour.x + neighbour.y * i32(image_size.x)] +  deposit_amount, 0.0, 1.0);
                }
            }
        }
    droplets[droplet_position].sediment -= amount;
}

fn erode(droplet_position: u32, amount: f32) {
    deposit(droplet_position, -amount);
}

fn fast_normal_approx(a: f32, x: f32) -> f32 {
    return a / (((0.1 * a + 1.0) * a) + x * x);
}

fn normal_curve(mean: f32, std_dev: f32, x: f32) -> f32 {
    let a = 1.0 / sqrt(std_dev * (2.0 * pi));
    let b = (-0.5) * pow((x - mean) / std_dev, 2.0);
    return a * exp(b);
}

fn bound_check_u32(position: vec2<u32>) -> bool {
    if position.x < image_size.x
        && position.x > 0
        && position.y < image_size.y
        && position.y > 0
        {
            return true;
    } else {
        return false;
    }
}

fn bound_check_i32(position: vec2<i32>) -> bool {
    if position.x < i32(image_size.x)
        && position.x > 0
        && position.y < i32(image_size.y)
        && position.y > 0
        {
            return true;
    } else {
        return false;
    }
}