// ==== FUN_00475950 @ 00475950 ====

undefined8
FUN_00475950(LPVOID param_1,undefined8 param_2,HMENU param_3,HMENU param_4,HMENU param_5,
            ulonglong *param_6,int param_7)

{
  ulonglong uVar1;
  ulonglong extraout_RAX;
  undefined8 uVar2;
  longlong lVar3;
  uint uVar4;
  short sVar5;
  ushort uVar6;
  uint uVar7;
  HMENU pHVar8;
  undefined6 uVar10;
  wchar_t *pwVar9;
  int local_58 [6];
  
  uVar7 = (uint)param_3;
  local_58[0] = 1;
  if (param_7 == 0) {
    uVar6 = (ushort)param_4;
    if (uVar7 == 6) {
      local_58[0] = 1;
      pHVar8 = param_4;
      uVar1 = FUN_004569e8((longlong)param_1,6,uVar6,param_5,local_58);
      goto LAB_004759c2;
    }
    pHVar8 = param_3;
    if (uVar7 == 0x11) {
      local_58[0] = 0;
      FUN_00461e54((longlong)param_1);
      *param_6 = 1;
LAB_004759c5:
      if (local_58[0] == 0) goto LAB_004759d0;
    }
    else {
      if (uVar7 == 0x2b1) {
        local_58[0] = 1;
        pHVar8 = param_4;
        uVar1 = FUN_00467ac0((longlong)param_1,0x2b1,(longlong)param_4,param_5,local_58);
LAB_004759c2:
        *param_6 = uVar1;
        goto LAB_004759c5;
      }
      if (uVar7 == 0xcb2) {
        local_58[0] = 1;
        pHVar8 = param_4;
        uVar1 = FUN_00474fac(param_1,0xcb2,param_4,(ulonglong)param_5,local_58);
        goto LAB_004759c2;
      }
LAB_004759d0:
      if (uVar7 == DAT_00551014) {
        local_58[0] = 1;
        pHVar8 = param_4;
        uVar1 = FUN_0046290c((longlong)param_1,(ulonglong)param_3 & 0xffffffff,(ulonglong)param_4,
                             (ulonglong)param_5,local_58);
        *param_6 = uVar1;
        if (local_58[0] != 0) goto LAB_00476440;
      }
      if (uVar7 == 0x4a) {
        local_58[0] = 1;
        uVar1 = FUN_0046ade8((longlong)param_1,0x4a,param_4,(longlong *)param_5);
LAB_00475a34:
        *param_6 = uVar1;
        if (local_58[0] != 0) goto LAB_00476440;
      }
      else {
        if (uVar7 == 0x31a) {
          local_58[0] = 1;
          uVar1 = FUN_0045751c((longlong)param_1);
          goto LAB_00475a34;
        }
        if (uVar7 == 0x407) {
LAB_00475b55:
          *param_6 = 0;
          goto LAB_00476440;
        }
        if (uVar7 == 0x111) {
          if ((ushort)(uVar6 + 0x4868) < 0x3e9) {
            pHVar8 = (HMENU)0x0;
            local_58[0] = 1;
            uVar1 = FUN_00474a34((longlong)param_1,(ulonglong)param_4 >> 0x10,uVar6,param_5,local_58
                                );
            *param_6 = uVar1;
            if (local_58[0] != 0) goto LAB_00476440;
          }
          if (uVar6 == 0xe141) {
            local_58[0] = 1;
            PostMessageW(*(HWND *)((longlong)param_1 + 8),0x10,0,0);
            *param_6 = 0;
LAB_00475bcb:
            if (local_58[0] != 0) goto LAB_00476440;
          }
          else {
            uVar10 = (undefined6)((ulonglong)pHVar8 >> 0x10);
            if (uVar6 == 0xe100) {
              local_58[0] = 1;
              uVar1 = FUN_00462814((longlong)param_1);
LAB_004760d2:
              *param_6 = uVar1;
              goto LAB_00475bcb;
            }
            uVar2 = CONCAT62(uVar10,0xe103);
            if ((uVar6 == 0xe103) || (uVar2 = CONCAT62(uVar10,0x8034), uVar6 == 0x8034)) {
              local_58[0] = 1;
              param_4 = (HMENU)FUN_0046ab8c((longlong)param_1,(ulonglong)param_4 >> 0x10,uVar2,
                                            (ulonglong)param_5);
              uVar1 = extraout_RAX;
              goto LAB_004760d2;
            }
            if (uVar6 == 0xe101) {
              local_58[0] = 1;
              uVar1 = FUN_00470a88((longlong)param_1);
              goto LAB_004760d2;
            }
            sVar5 = -0x4cde;
            if (((uVar6 == 0xb322) || (sVar5 = -0x428b, uVar6 == 0xbd75)) ||
               (sVar5 = -0x4289, uVar6 == 0xbd77)) {
              local_58[0] = 1;
              uVar1 = FUN_004626b8((longlong)param_1,(ulonglong)param_4 >> 0x10,sVar5);
              goto LAB_004760d2;
            }
            if (uVar6 == 0xe800) goto LAB_00475b55;
            if (uVar6 == 0xe801) {
              local_58[0] = 1;
              uVar1 = FUN_00468d0c((longlong)param_1);
              goto LAB_004760d2;
            }
            pwVar9 = (wchar_t *)CONCAT62(uVar10,0xe140);
            if ((((uVar6 == 0xe140) ||
                 (pwVar9 = (wchar_t *)CONCAT62(uVar10,0xe147), uVar6 == 0xe147)) ||
                ((pwVar9 = (wchar_t *)CONCAT62(uVar10,0xb3ae), uVar6 == 0xb3ae ||
                 ((pwVar9 = (wchar_t *)CONCAT62(uVar10,0xb253), uVar6 == 0xb253 ||
                  (pwVar9 = (wchar_t *)CONCAT62(uVar10,0x800e), uVar6 == 0x800e)))))) ||
               ((pwVar9 = (wchar_t *)CONCAT62(uVar10,0x808f), uVar6 == 0x808f ||
                ((((((pwVar9 = (wchar_t *)CONCAT62(uVar10,0x800d), uVar6 == 0x800d ||
                     (pwVar9 = (wchar_t *)CONCAT62(uVar10,0x800c), uVar6 == 0x800c)) ||
                    (pwVar9 = (wchar_t *)CONCAT62(uVar10,0x800f), uVar6 == 0x800f)) ||
                   ((pwVar9 = (wchar_t *)CONCAT62(uVar10,0x852c), uVar6 == 0x852c ||
                    (pwVar9 = (wchar_t *)CONCAT62(uVar10,0x8593), uVar6 == 0x8593)))) ||
                  (pwVar9 = (wchar_t *)CONCAT62(uVar10,0xe146), uVar6 == 0xe146)) ||
                 (pwVar9 = (wchar_t *)CONCAT62(uVar10,0x802f), uVar6 == 0x802f)))))) {
LAB_00475d29:
              local_58[0] = 1;
              uVar1 = FUN_00463cf4((longlong)param_1,(ulonglong)param_4 >> 0x10,pwVar9,
                                   (ulonglong)param_5);
              goto LAB_004760d2;
            }
            if (uVar6 == 0xa770) {
              local_58[0] = 1;
              uVar1 = FUN_0045b29c((longlong)param_1);
              goto LAB_004760d2;
            }
            if (uVar6 == 0x85b2) {
              local_58[0] = 1;
              uVar1 = FUN_00455ff4((longlong)param_1);
              goto LAB_004760d2;
            }
            pwVar9 = (wchar_t *)CONCAT62(uVar10,0x8030);
            if (((uVar6 == 0x8030) || (pwVar9 = (wchar_t *)CONCAT62(uVar10,0x8031), uVar6 == 0x8031)
                ) || (pwVar9 = (wchar_t *)CONCAT62(uVar10,0x8032), uVar6 == 0x8032))
            goto LAB_00475d29;
            sVar5 = -0x7fbb;
            if ((uVar6 == 0x8045) || (sVar5 = -0x7fba, uVar6 == 0x8046)) {
              local_58[0] = 1;
              uVar1 = FUN_0045abc0(param_1,(ulonglong)param_4 >> 0x10,sVar5,param_5,
                                   (undefined8 *)local_58);
              goto LAB_004760d2;
            }
            sVar5 = -0x7f94;
            if ((((uVar6 == 0x806c) || (sVar5 = -0x7f93, uVar6 == 0x806d)) ||
                (sVar5 = -0x7f95, uVar6 == 0x806b)) ||
               (((sVar5 = -0x7f96, uVar6 == 0x806a || (sVar5 = 0x413, uVar6 == 0x413)) ||
                ((sVar5 = 0x415, uVar6 == 0x415 || (sVar5 = 0x414, uVar6 == 0x414)))))) {
              local_58[0] = 1;
              uVar1 = FUN_0046dc10((longlong)param_1,(ulonglong)param_4 >> 0x10,sVar5,param_5,
                                   (LPCITEMIDLIST)local_58);
              goto LAB_004760d2;
            }
            if ((0x80bf < uVar6) && (uVar6 < 0x80cd)) {
              local_58[0] = 1;
              uVar1 = FUN_0045b114((longlong)param_1,(ulonglong)param_4 >> 0x10,uVar6);
              *param_6 = uVar1;
              if (local_58[0] != 0) goto LAB_00476440;
            }
            if ((4999 < uVar6) && (uVar6 < 0x2329)) {
              local_58[0] = 1;
              uVar1 = FUN_0045b224((longlong)param_1,(ulonglong)param_4 >> 0x10,uVar6,param_5,
                                   local_58);
              *param_6 = uVar1;
              if (local_58[0] != 0) goto LAB_00476440;
            }
            if ((0x8018 < uVar6) && (uVar6 < 0x802d)) {
              local_58[0] = 1;
              uVar1 = FUN_00467268((longlong)param_1,(ulonglong)param_4 >> 0x10,uVar6,
                                   (ulonglong)param_5,local_58);
              *param_6 = uVar1;
              if (local_58[0] != 0) goto LAB_00476440;
            }
            if (uVar6 == 0x8504) {
              local_58[0] = 1;
              uVar1 = FUN_00467268((longlong)param_1,(ulonglong)param_4 >> 0x10,0x8504,
                                   (ulonglong)param_5,local_58);
              goto LAB_004760d2;
            }
            if ((0x8489 < uVar6) && (uVar6 < 0x8492)) {
              local_58[0] = 1;
              uVar1 = FUN_0045640c(param_1,(ulonglong)param_4 >> 0x10,uVar6,param_5,local_58);
              *param_6 = uVar1;
              if (local_58[0] != 0) goto LAB_00476440;
            }
            if ((0x8040 < uVar6) && (uVar6 < 0x8045)) {
              local_58[0] = 1;
              uVar1 = FUN_00456670((longlong)param_1,(ulonglong)param_4 >> 0x10,
                                   (uint)param_4 & 0xffff,param_5,local_58);
              *param_6 = uVar1;
              if (local_58[0] != 0) goto LAB_00476440;
            }
            if ((0x8496 < uVar6) && (uVar6 < 0x84a3)) {
              local_58[0] = 1;
              uVar1 = FUN_0046bd8c((longlong)param_1,(ulonglong)param_4 >> 0x10,
                                   (uint)param_4 & 0xffff,param_5,local_58);
              goto LAB_004760d2;
            }
          }
          local_58[0] = 1;
          uVar1 = FUN_00470c30((longlong)param_1,0x111,(LPCWSTR)param_4,(LPCWSTR)param_5,local_58);
          goto LAB_00475a34;
        }
        uVar4 = 0x120;
        if ((((uVar7 == 0x120) || (uVar4 = 0x2c, uVar7 == 0x2c)) || (uVar4 = 0x11f, uVar7 == 0x11f))
           || (uVar4 = 0x2b, uVar7 == 0x2b)) {
LAB_004760e3:
          local_58[0] = 1;
          uVar1 = FUN_00455048((longlong)param_1,uVar4,param_4,param_5,local_58);
          goto LAB_00475a34;
        }
        if (uVar7 == 0x117) {
          uVar4 = 0x117;
          goto LAB_004760e3;
        }
        if (uVar7 == 0x4e) {
          local_58[0] = 1;
          uVar1 = FUN_00455ad0((longlong)param_1,0x4e,param_4,(undefined8 *)param_5,local_58);
          goto LAB_00475a34;
        }
        if (uVar7 == 0x233) {
          local_58[0] = 0;
          *param_6 = 0;
        }
        else if (uVar7 == 0x8064) {
          local_58[0] = 1;
          uVar1 = FUN_00464f3c((longlong)param_1,0x8064,param_4,(longlong)param_5);
          goto LAB_00475a34;
        }
      }
      if (uVar7 == DAT_00551010) {
        local_58[0] = 1;
        uVar1 = FUN_00456300((longlong)param_1);
        *param_6 = uVar1;
        if (local_58[0] != 0) goto LAB_00476440;
      }
      if (uVar7 == 2) {
        local_58[0] = 1;
        uVar1 = FUN_00457d88((longlong)param_1);
LAB_00475aa1:
        *param_6 = uVar1;
        if (local_58[0] != 0) goto LAB_00476440;
      }
      else {
        if (uVar7 == 0x10) {
          local_58[0] = 1;
          uVar1 = FUN_00462024((longlong)param_1);
          goto LAB_00475aa1;
        }
        if (uVar7 == 0x4e) {
          if (param_5[4].unused == -0x212) {
            local_58[0] = 1;
            uVar1 = FUN_00456e10((longlong)param_1,(UINT)param_4,(longlong)param_5);
            *param_6 = uVar1;
            if (local_58[0] != 0) goto LAB_00476440;
          }
          if (param_5[4].unused == -0x2c6) {
            local_58[0] = 1;
            uVar1 = FUN_004743e4((longlong)param_1,(ulonglong)param_4 & 0xffffffff,(longlong)param_5
                                 ,local_58);
            goto LAB_00475aa1;
          }
        }
        else {
          if (uVar7 == 0x112) {
            local_58[0] = 1;
            uVar1 = FUN_004553a0((longlong)param_1,0x112,(longlong)param_4,param_5,local_58);
            goto LAB_00475aa1;
          }
          if (uVar7 == 0x20) {
            local_58[0] = 0;
            *param_6 = 0;
          }
        }
      }
      uVar2 = FUN_0042f58c((longlong)param_1 + 0x78,param_2,uVar7,param_4,param_5,param_6,0);
      if (((int)uVar2 == 0) &&
         (uVar2 = FUN_0046f504((longlong)param_1,param_2,uVar7,(ulonglong)param_4,(longlong)param_5,
                               param_6,0), (int)uVar2 == 0)) {
        if (uVar7 == 0x111) {
          sVar5 = (short)param_4;
          if (sVar5 == -0x7fa3) {
            local_58[0] = 1;
            uVar1 = FUN_0045a30c((longlong)param_1);
LAB_004762dd:
            *param_6 = uVar1;
          }
          else if (sVar5 == -0x7fd3) {
            local_58[0] = 1;
            lVar3 = FUN_00456560((longlong)param_1);
            if (lVar3 != 0) {
              FUN_004c0b24(lVar3);
            }
            *param_6 = 1;
          }
          else {
            if (sVar5 != -0x7fee) {
              uVar6 = 0x8495;
              if ((sVar5 != -0x7b6b) && (uVar6 = 0x8494, sVar5 != -0x7b6c)) goto LAB_004763bf;
              local_58[0] = 1;
              uVar1 = FUN_0045a6a0((longlong)param_1,(ushort)((ulonglong)param_4 >> 0x10),uVar6,
                                   param_5,local_58);
              goto LAB_004762dd;
            }
            local_58[0] = 1;
            FUN_0045a6f0((longlong)param_1);
            *param_6 = 0;
          }
          if (local_58[0] != 0) goto LAB_00476440;
        }
        else if (uVar7 == 7) {
          local_58[0] = 1;
          if (*(HWND *)((longlong)param_1 + 0x50) != (HWND)0x0) {
            SetFocus(*(HWND *)((longlong)param_1 + 0x50));
          }
          *param_6 = 1;
        }
        else {
          if (uVar7 == 0x117) {
            local_58[0] = 1;
            uVar1 = FUN_004756fc((longlong)param_1,0x117,param_4,(ulonglong)param_5,local_58);
            goto LAB_004762dd;
          }
          if (uVar7 == 0x116) {
            *param_6 = 0;
          }
        }
LAB_004763bf:
        local_58[0] = 1;
        uVar1 = FUN_0041ab68((longlong)param_1,uVar7,(WPARAM)param_4,(HWND)param_5,local_58);
        *param_6 = uVar1;
        if (local_58[0] == 0) {
          if (uVar7 == 0x113) {
            local_58[0] = 1;
            uVar1 = FUN_00473cd8((longlong)param_1,0x113,(ulonglong)param_4,(ulonglong)param_5);
          }
          else {
            if (uVar7 != 0x7b) goto LAB_00476445;
            local_58[0] = 1;
            uVar1 = FUN_0046aae4((longlong)param_1,0x7b,param_4,param_5,(tagPOINT)local_58);
          }
          *param_6 = uVar1;
          if (local_58[0] == 0) goto LAB_00476445;
        }
      }
    }
LAB_00476440:
    uVar2 = 1;
  }
  else {
LAB_00476445:
    uVar2 = 0;
  }
  return uVar2;
}


