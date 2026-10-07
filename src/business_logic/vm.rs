use axum::{Extension, Json};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum_anyhow::{not_found, ApiResult, conflict, bad_request};
use gns3fy_rs::{ConsoleType, Lookup, Node, NodeStatus, QemuTemplate, Template, TemplateKind};
use sea_orm::{EntityTrait};
use serde::{Deserialize};
use uuid::Uuid;
use crate::{CONNECTOR, info};
use crate::models::image;
use crate::models::image::Architecture;
use crate::models::user::User;
use crate::models::vm::Vm;
use crate::server::ServerState;
use crate::utils::gns3::image::find_gns3_image;
use crate::utils::gns3::template::{get_template_from_node, template_name};
use crate::utils::user::get_user_project;

const TARGET: &str = "VM";

#[derive(Deserialize)]
pub struct CreateVmRequest {
    name: String,
    cpu: u64,
    ram: u64,
    image_id: Uuid
}

pub async fn create_vm(State(state): State<ServerState>, Extension(user): Extension<User>, Json(body): Json<CreateVmRequest>) -> ApiResult<Json<Vm>> {
    let image = image::Entity::find_by_id(body.image_id)
        .one(&state.orm)
        .await?
        .ok_or(not_found("Not found", "Image does not exist"))?;

    let gns3_image = find_gns3_image(&image).await?;

    let qemu_command = match &image.architecture {
        Architecture::X86 | Architecture::X86_64 => "qemu-system-x86_64",
        Architecture::ARM64 => "qemu-system-arm"
    };

    let qemu_template = QemuTemplate {
        qemu_path: Some(String::from(qemu_command)),
        platform: None,
        ram: Some(body.ram as i64),
        cpus: Some(body.cpu as i64),
        adapters: Some(6),
        adapter_type: Some(String::from("virtio-net-pci")),
        mac_address: None,
        first_port_name: None,
        port_name_format: Some(String::from("Ethernet{0}")),
        port_segment_size: Some(0),
        custom_adapters: None,
        options: Some(String::from("-cpu host")),
        boot_priority: Some(String::from("c")),
        on_close: Some(String::from("power_off")),
        process_priority: Some(String::from("normal")),
        cpu_throttling: Some(0),
        legacy_networking: Some(false),
        linked_clone: Some(true),
        bios_image: None,
        cdrom_image: None,
        initrd: None,
        kernel_image: None,
        kernel_command_line: None,
        hda_disk_image: Some(gns3_image.path),
        hda_disk_interface: Some(String::from("ide")),
        hdb_disk_image: None,
        hdb_disk_interface: None,
        hdc_disk_image: None,
        hdc_disk_interface: None,
        hdd_disk_image: None,
        hdd_disk_interface: None,
    };

    let template_name = template_name(&user.username, &body.name);
    let mut template = Template::new(CONNECTOR.clone(), &template_name, TemplateKind::Qemu(qemu_template))
        .with_compute_id("local")
        .with_category("guest")
        .with_symbol("linux_guest.svg");

    template.console_type = Some(ConsoleType::Vnc);
    template.console_auto_start = Some(false);
    template.default_name_format = Some(String::from("{name}-{0}"));
    template.builtin = Some(false);

    let connector = CONNECTOR.clone();
    if connector.get_template(Lookup::Name(&template_name)).await?.is_some() {
        return Err(conflict("Conflict", "A VM with the same name already exists"));
    }
    else {
        template.create().await?;
    }

    let project = get_user_project(&user.username).await?;

    let mut node = Node::with_connector(connector)
        .with_name(&body.name)
        .with_project_id(project.project_id.unwrap())
        .with_template_id(template.template_id.unwrap());

    node.create().await?;

    let vm = Vm::from_node(&node, &state.orm).await?;

    info!("Created VM \"{}\" for user {}", body.name, user.username);

    Ok(Json(vm))
}

pub async fn delete_vm(Extension(user): Extension<User>, Path(vm_name): Path<String>) -> ApiResult<(StatusCode, String)> {
    let connector = CONNECTOR.clone();
    let project = get_user_project(&user.username).await?;

    let mut nodes = connector
        .get_nodes(&project.project_id.unwrap())
        .await?;

    let node = nodes
        .iter_mut()
        .find(|n| n.name.as_ref() == Some(&vm_name))
        .ok_or(not_found("Not found", "VM not found"))?;

    match node.status {
        Some(NodeStatus::Started) | Some(NodeStatus::Suspended) => return Err(bad_request("Bad request", "Stop the VM first")),
        _ => {}
    }

    node.delete().await?;

    let mut template = get_template_from_node(&node).await?;

    template.delete().await?;

    info!("Deleted VM \"{}\" for user {}", vm_name, user.username);

    Ok((StatusCode::OK, String::from("VM deleted")))
}

pub async fn list_user_vms(State(state): State<ServerState>,Extension(user): Extension<User>) -> ApiResult<Json<Vec<Vm>>> {
    let connector = CONNECTOR.clone();
    let project = get_user_project(&user.username).await?;

    let mut vms = Vec::new();
    let nodes = connector.get_nodes(&project.project_id.unwrap()).await?;

    for node in nodes {
        let vm = Vm::from_node(&node, &state.orm).await?;
        vms.push(vm);
    }

    Ok(Json(vms))
}