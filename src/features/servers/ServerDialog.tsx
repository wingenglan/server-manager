import * as Dialog from "@radix-ui/react-dialog";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Eye, EyeOff, ShieldCheck, X } from "lucide-react";
import { type FormEvent, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../../components/ui/Button";
import { api } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import type { AuthType, SaveServerInput, ServerProfile, SudoMode } from "../../types/server";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  profile?: ServerProfile | null;
}

const initial: SaveServerInput = {
  name: "", description: "", host: "", port: 22, username: "root",
  authType: "password", sudoMode: "none", tags: [], favorite: false,
};

function formFor(profile?: ServerProfile | null): SaveServerInput {
  if (!profile) return initial;
  return {
    id: profile.id,
    name: profile.name,
    description: profile.description,
    host: profile.host,
    port: profile.port,
    username: profile.username,
    authType: profile.authType,
    privateKeyPath: profile.privateKeyPath ?? undefined,
    sudoMode: profile.sudoMode,
    groupId: profile.groupId ?? undefined,
    tags: profile.tags,
    favorite: profile.favorite,
  };
}

export function ServerDialog({ open, onOpenChange, profile }: Props) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const groups = useQuery({ queryKey: ["server-groups"], queryFn: api.listServerGroups });
  const [form, setForm] = useState(() => formFor(profile));
  const [showSecret, setShowSecret] = useState(false);
  const mutation = useMutation({
    mutationFn: api.saveServer,
    onSuccess: async (server) => {
      await queryClient.invalidateQueries({ queryKey: ["servers"] });
      setForm(formFor());
      onOpenChange(false);
      navigate(`/servers/${server.id}`);
    },
  });
  const set = <K extends keyof SaveServerInput>(key: K, value: SaveServerInput[K]) =>
    setForm((current) => ({ ...current, [key]: value }));
  const submit = (event: FormEvent) => {
    event.preventDefault();
    mutation.mutate({ ...form, name: form.name.trim(), host: form.host.trim(), username: form.username.trim() });
  };

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content className="dialog-content">
          <div className="dialog-header">
            <div><Dialog.Title>{profile ? "编辑 SSH 服务器" : "添加 SSH 服务器"}</Dialog.Title><Dialog.Description>连接信息保存在本机，密码只进入系统安全存储。</Dialog.Description></div>
            <Dialog.Close asChild><button className="icon-control" aria-label="关闭"><X size={17} /></button></Dialog.Close>
          </div>
          <form onSubmit={submit} className="server-form">
            <div className="field-grid field-grid--2">
              <label><span>显示名称</span><input required value={form.name} onChange={(e) => set("name", e.target.value)} placeholder="例如：生产 Web 01" /></label>
              <label><span>标签</span><input value={form.tags.join(",")} onChange={(e) => set("tags", e.target.value.split(",").map((v) => v.trim()).filter(Boolean))} placeholder="生产, 华东" /></label>
            </div>
            <div className="field-grid field-grid--host">
              <label><span>主机地址</span><input required value={form.host} onChange={(e) => set("host", e.target.value)} placeholder="192.0.2.10" /></label>
              <label><span>端口</span><input required type="number" min={1} max={65535} value={form.port} onChange={(e) => set("port", Number(e.target.value))} /></label>
            </div>
            <div className="field-grid field-grid--2">
              <label><span>用户名</span><input required value={form.username} onChange={(e) => set("username", e.target.value)} /></label>
              <label><span>认证方式</span><select value={form.authType} onChange={(e) => set("authType", e.target.value as AuthType)}><option value="password">密码</option><option value="private_key">私钥文件</option></select></label>
            </div>
            {form.authType === "password" && (
              <label><span>SSH 密码{profile ? "（留空则不修改）" : ""}</span><div className="secret-input"><input required={!profile} type={showSecret ? "text" : "password"} value={form.password ?? ""} onChange={(e) => set("password", e.target.value)} autoComplete="new-password" /><button type="button" onClick={() => setShowSecret((v) => !v)}>{showSecret ? <EyeOff size={16} /> : <Eye size={16} />}</button></div></label>
            )}
            {form.authType === "private_key" && <div className="field-grid field-grid--2"><label><span>私钥路径</span><input required value={form.privateKeyPath ?? ""} onChange={(e) => set("privateKeyPath", e.target.value)} placeholder="C:\\Users\\me\\.ssh\\id_ed25519" /></label><label><span>私钥口令{profile ? "（留空则不修改）" : ""}</span><input type="password" value={form.privateKeyPassphrase ?? ""} onChange={(e) => set("privateKeyPassphrase", e.target.value)} autoComplete="new-password" /></label></div>}
            <div className="field-grid field-grid--2">
              <label><span>sudo 模式</span><select value={form.sudoMode} onChange={(e) => set("sudoMode", e.target.value as SudoMode)}><option value="none">不使用 sudo</option><option value="passwordless">免密 sudo</option><option value="password">使用 sudo 密码</option></select></label>
              {form.sudoMode === "password" && <label><span>sudo 密码</span><input type="password" value={form.sudoPassword ?? ""} onChange={(e) => set("sudoPassword", e.target.value)} autoComplete="new-password" /></label>}
            </div>
            <label><span>服务器分组</span><select value={form.groupId ?? ""} onChange={(e) => set("groupId", e.target.value || undefined)}><option value="">未分组</option>{groups.data?.map((group) => <option key={group.id} value={group.id}>{group.name}</option>)}</select></label>
            <label className="check-field"><input type="checkbox" checked={form.favorite} onChange={(event) => set("favorite", event.target.checked)} /><span>加入收藏并置顶显示</span></label>
            <div className="security-note"><ShieldCheck size={18} /><span><strong>Host Key 校验默认开启</strong>首次连接会显示指纹，只有确认信任后才会认证。</span></div>
            {mutation.error && <div className="form-error">{errorMessage(mutation.error)}</div>}
            <div className="dialog-actions"><Dialog.Close asChild><Button type="button" variant="ghost">取消</Button></Dialog.Close><Button type="submit" variant="primary" disabled={mutation.isPending}>{mutation.isPending ? "保存中…" : profile ? "保存修改" : "保存并继续"}</Button></div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
