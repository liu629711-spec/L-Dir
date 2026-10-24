// ==== FUN_00429bc4 @ 00429bc4 ====

void FUN_00429bc4(longlong param_1)

{
  int iVar1;
  LPCWSTR pWVar2;
  undefined8 *puVar3;
  undefined8 *puVar4;
  longlong *plVar5;
  undefined8 uVar6;
  longlong lVar7;
  char **ppcVar8;
  char *pcVar9;
  int *piVar10;
  longlong local_res8;
  longlong local_res10;
  longlong local_res18;
  longlong local_res20;
  undefined **local_c8;
  undefined8 local_c0;
  undefined8 local_b8;
  longlong local_b0;
  longlong local_a8;
  longlong local_a0;
  longlong local_98 [2];
  char *local_88;
  char *local_80 [4];
  char *local_60;
  char *local_58;
  char *local_50;
  undefined8 local_48;
  
  FUN_0042728c();
  puVar4 = (undefined8 *)0x0;
  if (*(longlong *)(param_1 + 0x68) == 0) {
    puVar3 = (undefined8 *)FUN_0050e1b0(0x10);
    if (puVar3 != (undefined8 *)0x0) {
      puVar4 = FUN_00501108(puVar3);
    }
    *(undefined8 **)(param_1 + 0x68) = puVar4;
  }
  FUN_00427e70(*(longlong *)(param_1 + 0x68),0,*(HWND *)(param_1 + 8));
  pcVar9 = "z6.png";
  local_c8 = &PTR_FUN_00529c20;
  local_80[0] = "v6.png";
  local_b8 = 0;
  local_80[1] = "upF.png";
  local_88 = "z6.png";
  local_80[2] = "new.png";
  local_48 = 0;
  local_80[3] = "view.png";
  ppcVar8 = &local_88;
  local_60 = "update.png";
  puVar4 = (undefined8 *)(param_1 + 0x90);
  local_58 = "go.png";
  local_50 = "viewtree.png";
  do {
    local_c0 = 0;
    plVar5 = FUN_0040331c(&local_res10,"PNG");
    pWVar2 = (LPCWSTR)*plVar5;
    plVar5 = FUN_0040331c(&local_res8,pcVar9);
    FUN_004270d8((longlong)&local_c8,(LPCWSTR)*plVar5,pWVar2,(HMODULE)0x0);
    piVar10 = (int *)(local_res8 + -0xc);
    if (piVar10 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar10;
      *piVar10 = *piVar10 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_res8 + -0xc));
      }
    }
    piVar10 = (int *)(local_res10 + -0xc);
    if (piVar10 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar10;
      *piVar10 = *piVar10 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_res10 + -0xc));
      }
    }
    uVar6 = local_c0;
    ppcVar8 = ppcVar8 + 1;
    local_c0 = 0;
    pcVar9 = *ppcVar8;
    *puVar4 = uVar6;
    puVar4 = puVar4 + 1;
  } while (pcVar9 != (char *)0x0);
  plVar5 = FUN_0040331c(&local_res20,"PNG");
  pWVar2 = (LPCWSTR)*plVar5;
  plVar5 = FUN_0040331c(&local_res18,"dd_r.png");
  FUN_004270d8((longlong)&local_c8,(LPCWSTR)*plVar5,pWVar2,(HMODULE)0x0);
  piVar10 = (int *)(local_res18 + -0xc);
  if (piVar10 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *piVar10;
    *piVar10 = *piVar10 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0((LPVOID)(local_res18 + -0xc));
    }
  }
  piVar10 = (int *)(local_res20 + -0xc);
  if (piVar10 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar1 = *piVar10;
    *piVar10 = *piVar10 + -1;
    UNLOCK();
    if (iVar1 + -1 < 1) {
      thunk_FUN_0050e3a0((LPVOID)(local_res20 + -0xc));
    }
  }
  uVar6 = local_c0;
  local_c0 = 0;
  *(undefined8 *)(param_1 + 0x898) = uVar6;
  uVar6 = FUN_0044c87c();
  if ((char)uVar6 == '\0') {
    plVar5 = FUN_0040331c(&local_a8,"PNG");
    pWVar2 = (LPCWSTR)*plVar5;
    plVar5 = FUN_0040331c(&local_b0,"dd.png");
    FUN_004270d8((longlong)&local_c8,(LPCWSTR)*plVar5,pWVar2,(HMODULE)0x0);
    piVar10 = (int *)(local_b0 + -0xc);
    if (piVar10 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar10;
      *piVar10 = *piVar10 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_b0 + -0xc));
      }
    }
    piVar10 = (int *)(local_a8 + -0xc);
    if (piVar10 == (int *)PTR_DAT_00546008) goto LAB_00429f75;
    LOCK();
    iVar1 = *piVar10;
    *piVar10 = *piVar10 + -1;
    UNLOCK();
  }
  else {
    plVar5 = FUN_0040331c(&local_a0,"PNG");
    pWVar2 = (LPCWSTR)*plVar5;
    plVar5 = FUN_0040331c(local_98,"dd_w.png");
    FUN_004270d8((longlong)&local_c8,(LPCWSTR)*plVar5,pWVar2,(HMODULE)0x0);
    piVar10 = (int *)(local_98[0] + -0xc);
    if (piVar10 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar10;
      *piVar10 = *piVar10 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_98[0] + -0xc));
      }
    }
    piVar10 = (int *)(local_a0 + -0xc);
    if (piVar10 == (int *)PTR_DAT_00546008) goto LAB_00429f75;
    LOCK();
    iVar1 = *piVar10;
    *piVar10 = *piVar10 + -1;
    UNLOCK();
    local_a8 = local_a0;
  }
  if (iVar1 + -1 < 1) {
    thunk_FUN_0050e3a0((LPVOID)(local_a8 + -0xc));
  }
LAB_00429f75:
  uVar6 = local_c0;
  local_c0 = 0;
  *(undefined8 *)(param_1 + 0x890) = uVar6;
  FUN_00429ac0(param_1,0,0xa773,7,0);
  FUN_004299f0(param_1,0);
  FUN_00429ac0(param_1,0,0xa76c,0,0);
  FUN_00429ac0(param_1,0,0xa76d,1,0);
  FUN_004299f0(param_1,0);
  FUN_00429ac0(param_1,0,0xa771,2,0);
  FUN_00429ac0(param_1,0,0xa772,3,0);
  FUN_00429ac0(param_1,0,0xa775,4,0);
  lVar7 = FUN_00429ac0(param_1,0,0xa770,5,0);
  *(undefined4 *)(lVar7 + 0x40) = 0;
  lVar7 = FUN_00429ac0(param_1,0,0xb379,6,0);
  *(undefined4 *)(lVar7 + 0x40) = 1;
  *(undefined4 *)(*(longlong *)(param_1 + 0x8b0) + 0x3c4) = 1;
  FUN_00428828(param_1);
  FUN_00429270(param_1,0x2fbe,1);
  FUN_00429270(param_1,0x2fbe,0);
  puVar4 = (undefined8 *)(param_1 + 0x490);
  lVar7 = 0x80;
  do {
    puVar3 = FUN_00427370(puVar4[-0x80]);
    *puVar4 = puVar3;
    puVar4 = puVar4 + 1;
    lVar7 = lVar7 + -1;
  } while (lVar7 != 0);
  FUN_004278c0(&local_c8);
  return;
}


